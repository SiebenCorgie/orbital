use std::time::Instant;

use crossbeam::channel::Sender;
use egui::{epaint::CircleShape, InputState, Key, Painter, PointerButton, Response, Shape, Stroke};
use nih_plug_egui::egui::Pos2;
use serde_derive::{Deserialize, Serialize};

use crate::{
    com::{ComMsg, SolarState},
    osc::OscillatorBank,
};

use super::orbital::{ObjTy, Orbital};

#[derive(Serialize, Deserialize, Clone)]
pub struct SolarSystem {
    last_center: Pos2,
    #[serde(default = "Instant::now", skip)]
    last_update: Instant,
    orbitals: Vec<Orbital>,
    pub paused: bool,
}

impl SolarSystem {
    pub fn new() -> Self {
        SolarSystem {
            last_center: Pos2::ZERO,
            last_update: Instant::now(),
            orbitals: Vec::new(),
            paused: false,
        }
    }

    pub fn paint(&mut self, center: Pos2, painter: &Painter) {
        painter.add(Shape::Circle(CircleShape {
            center,
            radius: ObjTy::Sun.radius(),
            fill: ObjTy::Sun.color(),
            stroke: Stroke::none(),
        }));

        for orbital in self.orbitals.iter() {
            orbital.paint(painter);
        }
        self.last_center = center;
    }

    ///Handles input for the solar systems painting area.
    pub fn handle_response(
        &mut self,
        coms: &mut Sender<ComMsg>,
        response: &Response,
        input: &InputState,
    ) {
        self.paused |= input.key_down(Key::Space);
        //update hover if there is any
        if let Some(hp) = response.hover_pos() {
            for orb in &mut self.orbitals {
                let _pause = orb.on_hover(hp);
            }
        }
        if let Some(interaction_pos) = input.pointer.interact_pos() {
            //track if any click was taken
            let mut click_taken = false;

            if response.drag_started() {
                let slot_candidate = self.find_slot();
                for orbital in &mut self.orbitals {
                    if orbital.on_drag_start(interaction_pos, slot_candidate) {
                        click_taken = true;
                        break;
                    }
                }
            }

            if response.dragged() {
                for orbital in &mut self.orbitals {
                    let _pausing = orbital.on_drag(interaction_pos);
                }
            }

            if response.drag_released() {
                for orbital in &mut self.orbitals {
                    orbital.on_drag_end();
                }
            }

            //checkout response
            if response.clicked() && !click_taken {
                //try to find an not yet used osc row
                if let Some(slot) = self.find_slot() {
                    self.orbitals.push(Orbital::new_primary(
                        interaction_pos,
                        self.last_center,
                        slot,
                    ));
                }
            }

            let scroll_delta = input.scroll_delta.y / 1000.0;
            if scroll_delta != 0.0 {
                for orbital in &mut self.orbitals {
                    orbital.on_scroll(scroll_delta, interaction_pos);
                }
            }

            if input.pointer.button_released(PointerButton::Secondary) {
                for orbit in (0..self.orbitals.len()).rev() {
                    if self.orbitals[orbit].on_delete(interaction_pos) {
                        self.orbitals.remove(orbit);
                    }
                }
            }
        }

        //update inner animation, but only if not pausing
        let delta = self.last_update.elapsed().as_secs_f32();
        self.last_update = Instant::now();
        if !self.paused {
            for orb in &mut self.orbitals {
                orb.update(delta);
            }
        }

        //TODO handle breakdown
        let _ = coms.send(ComMsg::SolarState(self.get_solar_state()));
    }

    //builds the solar state from the current state. Used mainly to init
    // the synth when headless
    pub fn get_solar_state(&self) -> SolarState {
        let mut builder = SolarState {
            states: Vec::with_capacity(OscillatorBank::BANK_SIZE),
        };

        for orb in &self.orbitals {
            orb.build_solar_state(&mut builder, None);
        }

        builder
    }

    fn find_slot(&mut self) -> Option<usize> {
        //TODO: searching is currently garbage. Would be better to track.
        'search: for candidate in 0..OscillatorBank::OSC_COUNT {
            for o in &self.orbitals {
                if o.slot_take(candidate) {
                    continue 'search; //restart
                }
            }
            return Some(candidate);
        }

        //NOTE: unreachable... and the search is garbage anyways
        None
    }
}
