use std::{sync::Arc, f32::consts::PI, time::Instant};

use nih_plug_egui::egui::{Painter, Color32, Stroke, Vec2, Widget, Sense, Pos2, Shape, epaint::CircleShape, Key, PointerButton};

use crate::OrbitalParams;


const TWOPI: f32 = 2.0 * PI;

fn rotate_vec2(src: Vec2, angle: f32) -> Vec2{
    let cos = angle.cos();
    let sin = angle.sin();
    let v = Vec2 {
        x: src.x*cos - src.y*sin,
        y: src.x*sin + src.y*cos
    };

    v
}

#[derive(Clone, Copy)]
enum ObjTy{
    Sun,
    Planet,
    Moon,
    Astroid
}

impl ObjTy{
    ///Paints self.
    fn pain(&self, center: Pos2, highlight: bool, painter: &Painter){
        let mut shape = CircleShape{center, radius: self.radius(), stroke: Stroke::none(), fill: self.color()};
        if highlight{
            shape.stroke = Stroke::new(Orbital::ORBIT_LINE_FAT, Color32::WHITE);
        }
        painter.add(Shape::Circle(shape));
    }

    fn color(&self) -> Color32{
        match self {
            ObjTy::Sun => Color32::LIGHT_YELLOW,
            ObjTy::Astroid => Color32::LIGHT_RED,
            ObjTy::Moon => Color32::GRAY,
            ObjTy::Planet => Color32::LIGHT_BLUE,
        }
    }

    fn lower(&self) -> Self{
        match self {
            ObjTy::Sun => ObjTy::Planet,
            ObjTy::Planet => ObjTy::Moon,
            ObjTy::Moon => ObjTy::Astroid,
            ObjTy::Astroid => ObjTy::Astroid
        }
    }

    fn radius(&self) -> f32{
        match self {
            ObjTy::Sun => 22.0,
            ObjTy::Planet => 12.5,
            ObjTy::Moon => 9.5,
            ObjTy::Astroid => 7.0
        }
    }
}


enum Interaction{
    ///Child being dragged out
    DragNewChild{
        hash: u64,
        obj: ObjTy,
        //Location the "drag" event is currently at
        at: Pos2
    },
    DragPlanet{
        at: Pos2
    },
    DragOrbit{
        at: Pos2
    },
    None
}

impl Interaction{
    fn set_location(&mut self, parent_center: Pos2, to: Pos2){
        match self {
            Interaction::DragNewChild { hash, obj, at } => {
                *at = to;
            },
            Interaction::DragPlanet { at } => *at = to,
            Interaction::DragOrbit { at } => {
                *at = to
            },
            Interaction::None => {}
        }
    }

    fn is_none(&self) -> bool{
        if let Interaction::None = self{
            true
        }else{
            false
        }
    }
}

///Object in an orbit
struct Orbital{
    //center of orbit, usually parents location or
    // center of frame
    center: Pos2,
    //radius of orbit, basically pitch of the osc
    radius: f32,
    //offest on orbit. Translates to phase shift on osc. In radiant
    offset: f32,

    //current phase (in radiant) of this orbital.
    phase: f32,
    //speed of this orbital in units/sec.
    speed: f32,

    orbit_width: f32,
    planet_highlight: bool,

    interaction: Interaction,

    obj: ObjTy,
    hash: u64,
    children: Vec<Orbital>,
}

impl Orbital{

    const HANDLE_WIDTH: f32 = 5.0;
    const OBJSIZE: f32 = 10.0;

    const ORBIT_LINE_WIDTH: f32 = 1.0;
    const ORBIT_LINE_FAT: f32 = 2.0;

    const ZERO_SHIFT: Vec2 = Vec2{x: 0.0, y: -1.0};

    fn new_primary(at: Pos2, center: Pos2) -> Self{

        let radius = (at - center).length();
        //find angle in a way that it is placed at this location.
        let offset = 0.0;
        let mut new_orb = Orbital {
            center,
            radius,
            orbit_width: Self::ORBIT_LINE_WIDTH,
            planet_highlight: false,

            phase: 0.0,
            speed: 1.0,

            offset,
            obj: ObjTy::Planet,
            interaction: Interaction::None,
            hash: 0,
            children: Vec::new()
        };

        new_orb.offset_to(at);

        new_orb
    }

    fn paint(&self, painter: &Painter){
        //paint orbit
        painter.add(Shape::Circle(CircleShape{
            radius: self.radius,
            center: self.center,
            stroke: Stroke::new(self.orbit_width, Color32::WHITE),
            fill: Color32::TRANSPARENT
        }));

        for c in &self.children{
            c.paint(painter);
        }

        //if currently dragging out a new one, draw that
        if let Interaction::DragNewChild { hash, obj, at } = &self.interaction{
            //build a temp object and paint that
            let mut tmp = Orbital::new_primary(*at, self.obj_pos());
            tmp.obj = *obj;
            tmp.hash = *hash;
            tmp.paint(painter);
        }

        self.obj.pain(self.obj_pos(), self.planet_highlight, painter);
    }

    fn obj_pos(&self) -> Pos2{
        self.center + rotate_vec2(Self::ZERO_SHIFT, (self.offset + self.phase) % TWOPI) * self.radius
    }


    ///Offsets self in a way that it is as close as possible to `look_at`.
    fn offset_to(&mut self, look_at: Pos2){

        let angle = {
            //we currently do that by shifting origin to center, constructing the "zero shift" vector and the
            // "to at" vector and getting the angle between those.
            let at_prime = look_at - self.center;
            let angle = (Self::ZERO_SHIFT.dot(at_prime) / (at_prime.length() * Self::ZERO_SHIFT.length())).acos();
            if look_at.x < self.center.x{
                TWOPI - angle
            }else{
                angle
            }
        };

        self.offset = angle;
    }

    fn update(&mut self, delta: f32) {
        self.phase = (self.phase + (self.speed * delta)) % TWOPI;
        let new_loc = self.obj_pos();
        for c in &mut self.children{
            //forward update center...
            c.center = new_loc;
            //..then call inner update
            c.update(delta);
        }
    }

    fn on_drag_start(&mut self, at: Pos2) -> bool{
        let used = match (self.is_on_orbit_handle(at), self.is_on_planet(at)){
            (false, true) => {
                //drag start on planet, start dragging out a child
                self.interaction = Interaction::DragNewChild { hash: 0, obj: self.obj.lower(), at };
                true
            },
            (true, true) => {
                self.interaction = Interaction::DragPlanet { at };
                self.phase = 0.0;
                true
            }
            (true, false) => {
                //dragging orbit, change orbit radius
                self.interaction = Interaction::DragOrbit { at };
                true
            },
            _ => {
                false
            }
        };

        //if unused, recurse
        if !used{
            for c in &mut self.children{
                if c.on_drag_start(at){
                    return true;
                }
            }
        }

        false
    }

    ///handles a drag event. Used with drag_start and release. Returns true if it was used
    fn on_drag(&mut self, drag_to: Pos2) -> bool{
        if !self.interaction.is_none(){
            self.interaction.set_location(self.obj_pos(), drag_to);

            //if we are dragging the orbit, or the planet, update base location.
            match self.interaction{
                Interaction::DragOrbit { at } => {
                    let new_rad = (self.center.to_vec2() - at.to_vec2()).length();
                    self.radius = new_rad;
                    let new_center = self.obj_pos();
                    for c in &mut self.children{
                        c.update_center(new_center);
                    }
                }
                Interaction::DragPlanet { at } => {
                    self.offset_to(at);
                    let new_center = self.obj_pos();
                    for c in &mut self.children{
                        c.update_center(new_center);
                    }
                },
                _ => {}
            }

            true
        }else{
            for c in &mut self.children{
                if c.on_drag(drag_to){
                    return true;
                }
            }
            false
        }
    }

    fn on_scroll(&mut self, delta: f32, at: Pos2){
        if self.is_on_orbit_handle(at){
            self.speed = (self.speed + delta).clamp(0.001, 100.0);
        }
        for c in &mut self.children{
            c.on_scroll(delta, at);
        }
    }

    fn update_center(&mut self, new_center: Pos2){
        self.center = new_center;
        let new_child_center = self.obj_pos();
        for c in &mut self.children{
            c.update_center(new_child_center);
        }
    }

    fn on_drag_end(&mut self){
        if !self.interaction.is_none(){
            match &self.interaction {
                Interaction::DragNewChild { hash, obj, at } => {
                    let mut child = Orbital::new_primary(*at, self.obj_pos());
                    child.hash = *hash;
                    child.obj = *obj;
                    self.children.push(child);
                    self.interaction = Interaction::None;
                },
                Interaction::DragOrbit { at: _ } => {
                    self.interaction = Interaction::None;
                },
                Interaction::DragPlanet { at: _ } => {
                    self.interaction = Interaction::None;
                }
                Interaction::None => {}
            }
        }

        //always pass release event down
        for c in &mut self.children{
            c.on_drag_end();
        }

    }

    fn is_on_orbit_handle(&self, loc: Pos2) -> bool{
        let handle_rad = (loc-self.center).length();
        handle_rad > (self.radius - Self::HANDLE_WIDTH) && handle_rad < (self.radius + Self::HANDLE_WIDTH)
    }

    fn is_on_planet(&self, loc: Pos2) -> bool{
        //TODO currently calculating "left side" on
        let pos = self.obj_pos();

        let rad = (loc-pos).length();
        rad < (Self::OBJSIZE + Self::HANDLE_WIDTH)
    }

    ///Notifies that cursor is hovering, returns true if hovering over anything interactable.
    fn on_hover(&mut self, at: Pos2) -> bool{

        let mut is_interactable = false;
        //only add hover effect if not dragging already
        if self.interaction.is_none(){
            //if hovering over our orbit, thicken line
            match (self.is_on_orbit_handle(at), self.is_on_planet(at)){
                (false, false) => {
                    //reset orbit render width
                    self.orbit_width = Self::ORBIT_LINE_WIDTH;
                    self.planet_highlight = false;
                },
                (true, true) => {
                    self.planet_highlight = true;
                    self.orbit_width = Self::ORBIT_LINE_FAT;
                    is_interactable = true;
                }
                (true, false) => {
                    //only on orbit, widen orbit line
                    self.orbit_width = Self::ORBIT_LINE_FAT;
                    self.planet_highlight = false;
                },
                (_, true) => {
                    //on planet, preffer over orbit.
                    self.orbit_width = Self::ORBIT_LINE_WIDTH;
                    self.planet_highlight = true;
                    is_interactable = true;
                }
            }
        }

        for c in &mut self.children{
            is_interactable |= c.on_hover(at);
        }
        is_interactable
    }
}

pub struct SolarSystem{
    hover_at: Pos2,

    orbitals: Vec<Orbital>,

}

impl SolarSystem{
    fn new() -> Self{
        SolarSystem{
            hover_at: Pos2::ZERO,
            orbitals: Vec::new(),
        }
    }
}

pub struct Renderer{
    pub params: Arc<OrbitalParams>,
    pub system: SolarSystem,
    pub last_update: Instant,
}

impl Widget for &mut Renderer{
    fn ui(self, ui: &mut nih_plug_egui::egui::Ui) -> nih_plug_egui::egui::Response {
        let rect = ui.clip_rect();
        let (response, painter) = ui.allocate_painter(rect.size(), Sense::click_and_drag());


        let mut pausing = ui.input().key_down(Key::Space);
        //update hover if there is any
        if let Some(hp) = response.hover_pos(){

            for orb in &mut self.system.orbitals{
                pausing |= orb.on_hover(hp);
            }
        }
        if let Some(interaction_pos) = ui.input().pointer.interact_pos(){
            //track if any click was taken
            let mut click_taken = false;

            if response.drag_started(){
                for orbital in &mut self.system.orbitals{
                    if orbital.on_drag_start(interaction_pos){
                        click_taken = true;
                        break;
                    }
                }
            }

            if response.dragged(){
                for orbital in &mut self.system.orbitals{
                    pausing |= orbital.on_drag(interaction_pos);
                }
            }

            if response.drag_released(){
                for orbital in &mut self.system.orbitals{
                    orbital.on_drag_end();
                }
            }

            //checkout response
            if response.clicked() && !click_taken{
                if let Some(pos) = ui.input().pointer.interact_pos(){
                    self.system.orbitals.push(Orbital::new_primary(pos, rect.center()));
                }
            }

            let scroll_delta = ui.input().scroll_delta.y / 100.0;
            if scroll_delta != 0.0{
                for orbital in &mut self.system.orbitals{
                    orbital.on_scroll(scroll_delta, interaction_pos);
                }
            }
        }

        //update inner animation, but only if not pausing
        let delta = self.last_update.elapsed().as_secs_f32();
        self.last_update = Instant::now();
        if !pausing{
            for orb in &mut self.system.orbitals{
                orb.update(delta);
            }
        }

        self.paint(rect.center(), &painter);

        response
    }
}

impl Renderer{
    pub fn new(params: Arc<OrbitalParams>) -> Self{
        Renderer {
            params,
            last_update: Instant::now(),
            system: SolarSystem::new()
        }
    }

    pub fn paint(&self, center: Pos2, painter: &Painter){
        painter.add(Shape::Circle(CircleShape {
            center,
            radius: ObjTy::Sun.radius(),
            fill: ObjTy::Sun.color(),
            stroke: Stroke::none()
        }));

        for orbital in self.system.orbitals.iter(){
            orbital.paint(painter);
        }

    }
}
