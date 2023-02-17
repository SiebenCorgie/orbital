use colorgrad::Gradient;
use nih_plug::nih_log;
use nih_plug_egui::egui::{epaint::CircleShape, Color32, Painter, Pos2, Shape, Stroke, Vec2};
use serde_derive::{Deserialize, Serialize};
use std::f32::consts::PI;

use crate::{
    com::{ModulatorState, PrimaryState, SolarState},
    osc::{
        modulator::{ModulatorOsc, ParentIndex},
        primary::PrimaryOsc,
    },
};

use super::solar_system::SlotAllocator;

//Lazy_static color ramps for the orbital types.
lazy_static::lazy_static! {
    static ref RAMP_SUN: Gradient = colorgrad::yl_or_rd();
    static ref RAMP_PLANET: Gradient = colorgrad::viridis();
    static ref RAMP_MOON: Gradient = colorgrad::CustomGradient::new().colors(&[
        colorgrad::Color::from_linear_rgba8(20, 20, 20, 255),
        colorgrad::Color::from_linear_rgba8(200, 200, 200, 255)
    ]).build().unwrap();
    static ref RAMP_ASTROID: Gradient = colorgrad::inferno();
}

pub const TWOPI: f32 = 2.0 * PI;
pub fn rotate_vec2(src: Vec2, angle: f32) -> Vec2 {
    let cos = angle.cos();
    let sin = angle.sin();
    let v = Vec2 {
        x: src.x * cos - src.y * sin,
        y: src.x * sin + src.y * cos,
    };

    v
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub(super) enum ObjTy {
    Sun,
    Planet,
    Moon,
    Astroid,
}

impl ObjTy {
    ///Paints self.
    pub(super) fn paint(&self, speed_index: i32, center: Pos2, highlight: bool, painter: &Painter) {
        let mut shape = CircleShape {
            center,
            radius: self.radius(),
            stroke: Stroke::none(),
            fill: self.color(speed_index),
        };
        if highlight {
            shape.stroke = Stroke::new(Orbital::ORBIT_LINE_FAT, Color32::WHITE);
        }
        painter.add(Shape::Circle(shape));
    }

    pub(super) fn color(&self, speed_index: i32) -> Color32 {
        //map into linear rang -20..20
        let alpha = ((speed_index as f64 + 20.0) / 40.0).clamp(0.0, 1.0);

        let color = match self {
            ObjTy::Sun => RAMP_SUN.at(alpha),
            ObjTy::Astroid => RAMP_ASTROID.at(alpha),
            ObjTy::Moon => RAMP_MOON.at(alpha),
            ObjTy::Planet => RAMP_PLANET.at(alpha),
        }
        .to_rgba8();

        Color32::from_rgb(color[0], color[1], color[2])
    }

    pub(super) fn lower(&self) -> Self {
        match self {
            ObjTy::Sun => ObjTy::Planet,
            ObjTy::Planet => ObjTy::Moon,
            ObjTy::Moon => ObjTy::Astroid,
            ObjTy::Astroid => ObjTy::Astroid,
        }
    }

    pub(super) fn radius(&self) -> f32 {
        match self {
            ObjTy::Sun => 22.0,
            ObjTy::Planet => 12.5,
            ObjTy::Moon => 9.5,
            ObjTy::Astroid => 7.0,
        }
    }

    pub fn is_secondary(&self) -> bool {
        match self {
            ObjTy::Sun | ObjTy::Planet => false,
            _ => true,
        }
    }

    pub fn max_orbit(&self) -> f32 {
        if self.is_secondary() {
            Orbital::MAX_ORBIT_SEC
        } else {
            Orbital::MAX_ORBIT_PRIM
        }
    }
}

#[derive(Clone)]
enum Interaction {
    ///Child being dragged out
    DragNewChild {
        slot: usize,
        obj: ObjTy,
        //Location the "drag" event is currently at
        at: Pos2,
    },
    DragPlanet {
        at: Pos2,
    },
    DragOrbit {
        at: Pos2,
    },
    None,
}

impl Default for Interaction {
    fn default() -> Self {
        Interaction::None
    }
}

impl Interaction {
    fn set_location(&mut self, to: Pos2) {
        match self {
            Interaction::DragNewChild {
                slot: _,
                obj: _,
                at,
            } => {
                *at = to;
            }
            Interaction::DragPlanet { at } => *at = to,
            Interaction::DragOrbit { at } => *at = to,
            Interaction::None => {}
        }
    }

    fn is_none(&self) -> bool {
        if let Interaction::None = self {
            true
        } else {
            false
        }
    }
}

///Object in an orbit
#[derive(Serialize, Deserialize, Clone)]
pub struct Orbital {
    //center of orbit, usually parents location or
    // center of frame
    center: Pos2,
    //radius of orbit, basically pitch of the osc
    radius: f32,
    //offest on orbit. Translates to phase shift on osc. In radiant
    offset: f32,

    //current phase (in radiant) of this orbital.
    phase: f32,
    //abstract orbital speed.
    speed_index: i32,

    orbit_width: f32,
    planet_highlight: bool,

    #[serde(skip)]
    interaction: Interaction,

    obj: ObjTy,
    ///Depending on the ObjTy, maps 1:1 into the OscBank's primary or modulator banks
    pub osc_slot: usize,
    children: Vec<Orbital>,
}

impl Orbital {
    const HANDLE_WIDTH: f32 = 5.0;
    const OBJSIZE: f32 = 10.0;

    const ORBIT_LINE_WIDTH: f32 = 1.0;
    const ORBIT_LINE_FAT: f32 = 2.0;
    const MIN_ORBIT: f32 = 25.0;
    const MAX_ORBIT_SEC: f32 = 100.0;
    const MAX_ORBIT_PRIM: f32 = 300.0;
    const ZERO_SHIFT: Vec2 = Vec2 { x: 0.0, y: -1.0 };
    pub const ABS_BASE_FREQ: f32 = 440.0;

    pub fn new_primary(at: Pos2, center: Pos2, slot: usize) -> Self {
        let radius = (at - center).length();
        //find angle in a way that it is placed at this location.
        let offset = 0.0;
        let mut new_orb = Orbital {
            center,
            radius,
            orbit_width: Self::ORBIT_LINE_WIDTH,
            planet_highlight: false,

            phase: 0.0,
            speed_index: 0,

            offset,
            obj: ObjTy::Planet,
            interaction: Interaction::None,
            osc_slot: slot,
            children: Vec::new(),
        };

        new_orb.offset_to(at);

        new_orb
    }

    pub fn paint(&self, painter: &Painter) {
        //paint orbit
        painter.add(Shape::Circle(CircleShape {
            radius: self.radius,
            center: self.center,
            stroke: Stroke::new(self.orbit_width, Color32::WHITE),
            fill: Color32::TRANSPARENT,
        }));

        for c in &self.children {
            c.paint(painter);
        }

        //if currently dragging out a new one, draw that
        if let Interaction::DragNewChild { slot, obj, at } = &self.interaction {
            //build a temp object and paint that
            let mut tmp = Orbital::new_primary(*at, self.obj_pos(), *slot);
            tmp.obj = *obj;
            tmp.radius = tmp.radius.clamp(Self::MIN_ORBIT, tmp.obj.max_orbit());
            tmp.paint(painter);
        }

        self.obj.paint(
            self.speed_index,
            self.obj_pos(),
            self.planet_highlight,
            painter,
        );
    }

    fn obj_pos(&self) -> Pos2 {
        self.center
            + rotate_vec2(Self::ZERO_SHIFT, (self.offset + self.phase) % TWOPI) * self.radius
    }

    ///Offsets self in a way that it is as close as possible to `look_at`.
    fn offset_to(&mut self, look_at: Pos2) {
        let angle = {
            //we currently do that by shifting origin to center, constructing the "zero shift" vector and the
            // "to at" vector and getting the angle between those.
            let at_prime = look_at - self.center;
            let angle = (Self::ZERO_SHIFT.dot(at_prime)
                / (at_prime.length() * Self::ZERO_SHIFT.length()))
            .acos();
            if look_at.x < self.center.x {
                TWOPI - angle
            } else {
                angle
            }
        };

        self.offset = angle;
    }

    fn anim_speed(&self) -> f32 {
        //using offsetted speed sigmoid
        1.0 + (self.speed_index as f32 / (1.0 + (self.speed_index as f32).abs()))
    }

    pub fn update(&mut self) {
        self.phase = self.phase % TWOPI;
        let new_loc = self.obj_pos();
        for c in &mut self.children {
            //forward update center...
            c.center = new_loc;
            //..then call inner update
            c.update();
        }
    }

    pub fn update_anim(&mut self, delta: f32) {
        self.phase = (self.phase + (self.anim_speed() * delta)) % TWOPI;
        let new_loc = self.obj_pos();
        for c in &mut self.children {
            //forward update center...
            c.center = new_loc;
            //..then call inner update
            c.update_anim(delta);
        }
    }

    pub fn on_drag_start(&mut self, at: Pos2, slot_candidates: &mut Option<usize>) -> bool {
        let used = match (self.is_on_orbit_handle(at), self.is_on_planet(at)) {
            (false, true) => {
                //drag start on planet, start dragging out a child
                // try to take the candidate. If not possible it was already taken and we can ignore
                if let Some(slot) = slot_candidates.take() {
                    self.interaction = Interaction::DragNewChild {
                        slot,
                        obj: self.obj.lower(),
                        at,
                    };
                    true
                } else {
                    false
                }
            }
            (true, true) => {
                self.interaction = Interaction::DragPlanet { at };
                self.phase = 0.0;
                true
            }
            (true, false) => {
                //dragging orbit, change orbit radius
                self.interaction = Interaction::DragOrbit { at };
                true
            }
            _ => false,
        };

        //if unused, recurse
        if !used {
            for c in &mut self.children {
                if c.on_drag_start(at, slot_candidates) {
                    return true;
                }
            }
        }

        false
    }

    ///handles a drag event. Used with drag_start and release. Returns true if it was used
    pub fn on_drag(&mut self, drag_to: Pos2) -> bool {
        if !self.interaction.is_none() {
            self.interaction.set_location(drag_to);

            //if we are dragging the orbit, or the planet, update base location.
            match self.interaction {
                Interaction::DragOrbit { at } => {
                    let new_rad = (self.center.to_vec2() - at.to_vec2()).length();
                    self.radius = new_rad.clamp(Self::MIN_ORBIT, self.obj.max_orbit());
                    let new_center = self.obj_pos();
                    for c in &mut self.children {
                        c.update_center(new_center);
                    }
                }
                Interaction::DragPlanet { at } => {
                    self.offset_to(at);
                    let new_center = self.obj_pos();
                    for c in &mut self.children {
                        c.update_center(new_center);
                    }
                }
                _ => {}
            }

            true
        } else {
            for c in &mut self.children {
                if c.on_drag(drag_to) {
                    return true;
                }
            }
            false
        }
    }

    pub fn on_drag_end(&mut self) {
        if !self.interaction.is_none() {
            match &self.interaction {
                Interaction::DragNewChild { slot, obj, at } => {
                    nih_log!("adding {:?} @ {}", obj, slot);
                    let mut child = Orbital::new_primary(*at, self.obj_pos(), *slot);
                    child.radius = child.radius.clamp(Self::MIN_ORBIT, obj.max_orbit());
                    child.obj = *obj;
                    self.children.push(child);
                    self.interaction = Interaction::None;
                }
                Interaction::DragOrbit { at: _ } => {
                    self.interaction = Interaction::None;
                }
                Interaction::DragPlanet { at: _ } => {
                    self.interaction = Interaction::None;
                }
                Interaction::None => {}
            }
        }

        //always pass release event down
        for c in &mut self.children {
            c.on_drag_end();
        }
    }

    pub fn on_scroll(&mut self, delta: f32, at: Pos2) {
        if self.is_on_orbit_handle(at) {
            self.speed_index = if delta < 0.0 {
                self.speed_index - 1
            } else {
                self.speed_index + 1
            };
        }
        for c in &mut self.children {
            c.on_scroll(delta, at);
        }
    }

    pub fn update_center(&mut self, new_center: Pos2) {
        self.center = new_center;
        let new_child_center = self.obj_pos();
        for c in &mut self.children {
            c.update_center(new_child_center);
        }
    }

    fn is_on_orbit_handle(&self, loc: Pos2) -> bool {
        let handle_rad = (loc - self.center).length();
        handle_rad > (self.radius - Self::HANDLE_WIDTH)
            && handle_rad < (self.radius + Self::HANDLE_WIDTH)
    }

    fn is_on_planet(&self, loc: Pos2) -> bool {
        //TODO currently calculating "left side" on
        let pos = self.obj_pos();

        let rad = (loc - pos).length();
        rad < (Self::OBJSIZE + Self::HANDLE_WIDTH)
    }

    //checks if self should be deleted
    pub fn on_delete(&mut self, at: Pos2, allocator: &mut SlotAllocator) -> bool {
        if self.is_on_orbit_handle(at) || self.is_on_planet(at) {
            //deallocate children slots
            for c in &self.children {
                c.deallocat_all(allocator);
            }
            true
        } else {
            //reverse through children, delete all that report it
            for i in (0..self.children.len()).rev() {
                if self.children[i].on_delete(at, allocator) {
                    //remove the child from self, by definition this must be a modulator,
                    // so we remove it from the allocator as well.
                    let removed = self.children.remove(i);
                    allocator.free_mod(removed.osc_slot);
                }
            }

            false
        }
    }

    fn deallocat_all(&self, allocator: &mut SlotAllocator) {
        allocator.free_mod(self.osc_slot);
        for c in &self.children {
            c.deallocat_all(allocator);
        }
    }

    ///Notifies that cursor is hovering, returns true if hovering over anything interactable.
    pub fn on_hover(&mut self, at: Pos2) -> bool {
        let mut is_interactable = false;
        //only add hover effect if not dragging already
        if self.interaction.is_none() {
            //if hovering over our orbit, thicken line
            match (self.is_on_orbit_handle(at), self.is_on_planet(at)) {
                (false, false) => {
                    //reset orbit render width
                    self.orbit_width = Self::ORBIT_LINE_WIDTH;
                    self.planet_highlight = false;
                }
                (true, true) => {
                    self.planet_highlight = true;
                    self.orbit_width = Self::ORBIT_LINE_FAT;
                    is_interactable = true;
                }
                (true, false) => {
                    //only on orbit, widen orbit line
                    self.orbit_width = Self::ORBIT_LINE_FAT;
                    self.planet_highlight = false;
                }
                (_, true) => {
                    //on planet, preffer over orbit.
                    self.orbit_width = Self::ORBIT_LINE_WIDTH;
                    self.planet_highlight = true;
                    is_interactable = true;
                }
            }
        }

        for c in &mut self.children {
            is_interactable |= c.on_hover(at);
        }
        is_interactable
    }

    ///appends self and the children to the state, returns the index self was added at
    pub fn build_solar_state(&self, builder: &mut SolarState, parent_slot: Option<ParentIndex>) {
        if let Some(slot) = parent_slot {
            let dist = self.radius - Self::MIN_ORBIT;
            //linear blend in orbit range
            let range = dist / (self.obj.max_orbit() - Self::MIN_ORBIT);

            builder.modulator_states.push(ModulatorState {
                state: ModulatorOsc {
                    parent_osc_slot: slot,
                    speed_index: self.speed_index,
                    range: range.clamp(0.0, 1.0),
                    is_on: true,
                },
                offset: self.phase,
                slot: self.osc_slot,
            });
        } else {
            let volume = self.radius / (self.obj.max_orbit() - Self::MIN_ORBIT);

            builder.primary_states.push(PrimaryState {
                offset: self.phase,
                slot: self.osc_slot,
                state: PrimaryOsc {
                    speed_index: self.speed_index,
                    volume,
                    is_on: true,
                },
            });
        }

        //If we have a parent, pass self down as modulator, otherwise
        // as primary.
        let parent_slot = if parent_slot.is_some() {
            ParentIndex::Modulator(self.osc_slot)
        } else {
            ParentIndex::Primary(self.osc_slot)
        };

        if let Interaction::DragNewChild { slot, obj, at } = &self.interaction {
            //add new child already so we can hear it.
            let mut tmp = Orbital::new_primary(*at, self.obj_pos(), *slot);
            tmp.obj = *obj;
            tmp.radius = tmp.radius.clamp(Self::MIN_ORBIT, tmp.obj.max_orbit());
            tmp.build_solar_state(builder, Some(parent_slot));
        }

        //do same with children
        for c in &self.children {
            c.build_solar_state(builder, Some(parent_slot));
        }
    }
}
