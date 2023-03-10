use std::time::Instant;

use crossbeam::channel::Sender;
use egui::{epaint::CircleShape, InputState, Painter, PointerButton, Response, Shape, Stroke};
use nih_plug::nih_log;
use nih_plug_egui::egui::Pos2;
use serde_derive::{Deserialize, Serialize};

use crate::{
    com::{ComMsg, SolarState},
    osc::{modulator::ParentIndex, OscillatorBank},
};

use super::orbital::{ObjTy, Orbital};

#[derive(Serialize, Deserialize, Clone)]
pub struct SlotAllocator {
    primary_slots: [bool; OscillatorBank::PRIMARY_OSC_COUNT],
    mod_slots: [bool; OscillatorBank::MOD_OSC_COUNT],
}

impl Default for SlotAllocator {
    fn default() -> Self {
        SlotAllocator {
            primary_slots: [false; OscillatorBank::PRIMARY_OSC_COUNT],
            mod_slots: [false; OscillatorBank::MOD_OSC_COUNT],
        }
    }
}

impl SlotAllocator {
    fn allocate_primary(&mut self) -> Option<usize> {
        for (slot_idx, slot_state) in self.primary_slots.iter_mut().enumerate() {
            if !*slot_state {
                *slot_state = true;
                nih_log!("allocate primary on {}", slot_idx);
                return Some(slot_idx);
            }
        }

        None
    }

    pub fn free_primary(&mut self, slot: usize) {
        if slot < OscillatorBank::PRIMARY_OSC_COUNT {
            nih_log!("Free primary slot {}", slot);
            self.primary_slots[slot] = false
        }
    }

    fn allocate_mod(&mut self) -> Option<usize> {
        for (slot_idx, slot_state) in self.mod_slots.iter_mut().enumerate() {
            if !*slot_state {
                *slot_state = true;
                nih_log!("allocate mod on {}", slot_idx);
                return Some(slot_idx);
            }
        }

        None
    }

    pub fn free_mod(&mut self, slot: usize) {
        if slot < OscillatorBank::MOD_OSC_COUNT {
            nih_log!("Free mod slot {}", slot);
            self.mod_slots[slot] = false
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SolarSystem {
    last_center: Pos2,
    //Primary orbitals. Each child is by necessity a modulator
    orbitals: Vec<Orbital>,
    allocator: SlotAllocator,
    #[serde(default = "Instant::now", skip)]
    pub last_update: Instant,
    pub is_paused: bool,
    #[serde(skip)]
    pub selected: Option<ParentIndex>,

    //If set from the outside, makes sure everything is redrawn.
    #[serde(skip)]
    pub is_dirty: bool,
    //If set, adds a new child on next update
    #[serde(skip)]
    pub is_add_child: bool,
}

impl SolarSystem {
    pub fn new() -> Self {
        let mut sys = SolarSystem {
            last_center: Pos2::ZERO,
            orbitals: Vec::new(),
            allocator: SlotAllocator::default(),
            last_update: Instant::now(),
            is_paused: true,
            selected: None,
            is_dirty: false,
            is_add_child: false,
        };

        //setup a base system. New is only called if there is no state at all,
        // so that should be all right.
        sys.insert_primary(Pos2 { x: 50.0, y: 50.0 }, Pos2 { x: 100.0, y: 100.0 });

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
            fill: ObjTy::Sun.color(0.0), //TODO: Maybe animate based on currently played key?
            stroke: Stroke::none(),
        }));

        for orbital in self.orbitals.iter() {
            orbital.paint(painter, self.selected);
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
        //handle child add if needed.
        if self.is_add_child {
            self.is_add_child = false;
            if let Some(index) = self.allocator.allocate_mod() {
                if let Some(orb) = self.get_selected_orbital() {
                    orb.spawn_child(index);
                    self.selected = Some(ParentIndex::Modulator(index));
                } else {
                    self.allocator.free_mod(index);
                }
            }
        }

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
                let mut slot_candidate = self.allocator.allocate_mod();
                for orbital in &mut self.orbitals {
                    if let Some(taken_orbital) =
                        orbital.on_drag_start(interaction_pos, &mut slot_candidate)
                    {
                        //Mark as taken and setup internal "selected" state
                        click_taken = true;
                        self.selected = Some(taken_orbital);
                        break;
                    }
                }

                //If the candidate slot was not taken, free it again
                if let Some(candidate) = slot_candidate {
                    self.allocator.free_mod(candidate)
                }
            }

            if click_taken {
                draw_state_changed = true;
            }

            if response.dragged() {
                for orbital in &mut self.orbitals {
                    let _pausing = orbital.on_drag(interaction_pos);
                }
                draw_state_changed = true;
            }

            if response.drag_released() {
                for orbital in &mut self.orbitals {
                    if let Some(idx) = orbital.on_drag_end() {
                        self.selected = Some(idx);
                    }
                }
            }

            //checkout response
            if response.clicked() && !click_taken {
                //try to select anything with this click, if we can't, we spawn a new entity
                let mut select_any = false;
                for orbital in &mut self.orbitals {
                    if let Some(idx) = orbital.on_select(interaction_pos) {
                        select_any = true;
                        self.selected = Some(idx);
                        draw_state_changed = true;
                        break;
                    }
                }

                //try to find an not yet used osc row
                if !select_any {
                    self.insert_primary(interaction_pos, self.last_center);
                    draw_state_changed = true;
                }
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
                    if self.orbitals[orbit].on_delete(interaction_pos, &mut self.allocator) {
                        let removed_primary = self.orbitals.remove(orbit);
                        self.allocator.free_primary(removed_primary.osc_slot);
                    }
                }
                //If the current selected is now marked as free, remove selection state.
                match self.selected {
                    Some(ParentIndex::Primary(p)) => {
                        if !self.allocator.primary_slots[p] {
                            self.selected = None;
                        }
                    }
                    Some(ParentIndex::Modulator(m)) => {
                        if !self.allocator.mod_slots[m] {
                            self.selected = None;
                        }
                    }
                    _ => {}
                }
                draw_state_changed = true;
            }
        }

        //promote dirty flag to draw state changed flag and reset
        if self.is_dirty {
            self.is_dirty = false;
            draw_state_changed = true;
        }

        //update inner animation, but only if not pausing
        if !self.is_paused {
            let delta = self.last_update.elapsed().as_secs_f32();
            self.last_update = Instant::now();
            for orb in &mut self.orbitals {
                orb.update_anim(delta);
            }
        } else {
            if draw_state_changed {
                for orb in &mut self.orbitals {
                    orb.update();
                }
            }
        }

        if draw_state_changed {
            //TODO handle breakdown
            let _ = coms.send(ComMsg::StateChange(self.get_solar_state()));
        }
    }

    pub fn reset_anim_state(&mut self) {
        self.last_update = Instant::now();
    }

    pub fn insert_primary(&mut self, at: Pos2, center: Pos2) {
        let slot = if let Some(s) = self.allocator.allocate_primary() {
            s
        } else {
            return;
        };

        self.orbitals.push(Orbital::new_primary(at, center, slot));
        //set as selected.
        self.selected = Some(ParentIndex::Primary(slot));
    }

    //builds the solar state from the current state. Used mainly to init
    // the synth when headless
    pub fn get_solar_state(&self) -> SolarState {
        let mut builder = SolarState {
            primary_states: Vec::with_capacity(OscillatorBank::PRIMARY_OSC_COUNT),
            modulator_states: Vec::with_capacity(OscillatorBank::MOD_OSC_COUNT),
        };

        for orb in &self.orbitals {
            orb.build_solar_state(&mut builder, None);
        }

        builder
    }

    pub fn get_selected_orbital(&mut self) -> Option<&mut Orbital> {
        if let Some(index) = self.selected {
            for orbital in &mut self.orbitals {
                let res = orbital.find_index_mut(index);
                if res.is_some() {
                    return res;
                }
            }

            None
        } else {
            None
        }
    }
}
