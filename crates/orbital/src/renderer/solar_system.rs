use crossbeam::channel::Sender;
use egui::{epaint::CircleShape, InputState, Painter, PointerButton, Response, Shape, Stroke};
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
    orbitals: Vec<Orbital>,
}

impl SolarSystem {
    pub fn new() -> Self {
        let mut sys = SolarSystem {
            last_center: Pos2::ZERO,
            orbitals: Vec::new(),
        };

        //setup a base system. New is only called if there is no state at all,
        // so that should be all right.
        if let Some(slot) = sys.find_slot() {
            sys.orbitals.push(Orbital::new_primary(
                Pos2 { x: 50.0, y: 50.0 },
                Pos2 { x: 100.0, y: 100.0 },
                slot,
            ));
        }

        sys
    }

    pub fn paint(&mut self, center: Pos2, painter: &Painter) {
        if self.last_center != center {
            for orbital in &mut self.orbitals {
                orbital.update_center(center);
            }
        }

        painter.add(Shape::Circle(CircleShape {
            center,
            radius: ObjTy::Sun.radius(),
            fill: ObjTy::Sun.color(0), //TODO: Maybe animate based on currently played key?
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
        //update hover if there is any
        if let Some(hp) = response.hover_pos() {
            for orb in &mut self.orbitals {
                let _pause = orb.on_hover(hp);
            }
        }

        let mut draw_state_changed = false;
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

            if click_taken {
                draw_state_changed = true;
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
                draw_state_changed = true;
            }

            let scroll_delta = input.scroll_delta.y / 1000.0;
            if scroll_delta != 0.0 {
                for orbital in &mut self.orbitals {
                    orbital.on_scroll(scroll_delta, interaction_pos);
                }
                draw_state_changed = true;
            }

            if input.pointer.button_released(PointerButton::Secondary) {
                for orbit in (0..self.orbitals.len()).rev() {
                    if self.orbitals[orbit].on_delete(interaction_pos) {
                        self.orbitals.remove(orbit);
                    }
                }
                draw_state_changed = true;
            }
        }

        if draw_state_changed {
            for orb in &mut self.orbitals {
                orb.update();
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
