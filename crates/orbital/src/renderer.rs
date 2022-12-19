use std::{sync::Arc, f32::consts::PI};

use nih_plug_egui::egui::{Painter, Color32, Stroke, Vec2, Widget, Sense, Pos2, Shape, epaint::CircleShape, color_picker};

use crate::OrbitalParams;


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
    Planet,
    Moon,
    Astroid
}

impl ObjTy{
    ///Paints self.
    fn pain(&self, center: Pos2, painter: &Painter){
        let color = match self {
            ObjTy::Astroid => Color32::LIGHT_RED,
            ObjTy::Moon => Color32::GRAY,
            ObjTy::Planet => Color32::LIGHT_BLUE,
        };

        painter.add(Shape::circle_filled(center, 10.0, color));
    }

    fn lower(&self) -> Self{
        match self {
            ObjTy::Planet => ObjTy::Moon,
            ObjTy::Moon => ObjTy::Astroid,
            ObjTy::Astroid => ObjTy::Astroid
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
    DragOrbit{
        new_orbit: f32
    },
    None
}

impl Interaction{
    fn set_location(&mut self, parent_center: Pos2, to: Pos2){
        match self {
            Interaction::DragNewChild { hash, obj, at } => {
                *at = to;
            },
            Interaction::DragOrbit { new_orbit } => {
                let orbit = (parent_center - to).length();
                *new_orbit = orbit;
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

    orbit_width: f32,

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
        let offset = {
            //we currently do that by shifting origin to center, constructing the "zero shift" vector and the
            // "to at" vector and getting the angle between those.
            let at_prime = at - center;
            let angle = (Self::ZERO_SHIFT.dot(at_prime) / (at_prime.length() * Self::ZERO_SHIFT.length())).acos();
            if at.x < center.x{
                2.0*PI - angle
            }else{
                angle
            }
        };
        Orbital {
            center,
            radius,
            orbit_width: Self::ORBIT_LINE_WIDTH,
            offset,
            obj: ObjTy::Planet,
            interaction: Interaction::None,
            hash: 0,
            children: Vec::new()
        }
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

        self.obj.pain(self.obj_pos(), painter);
    }

    fn obj_pos(&self) -> Pos2{
        self.center + rotate_vec2(Self::ZERO_SHIFT, self.offset) * self.radius
    }


    fn on_drag_start(&mut self, at: Pos2) -> bool{
        let used = match (self.on_orbit_handle(at), self.on_planet(at)){
            (_, true) => {
                //drag start on planet, start dragging out a child
                self.interaction = Interaction::DragNewChild { hash: 0, obj: self.obj.lower(), at };

                true
            }
            (true, false) => {
                //dragging orbit, change orbit radius
                self.interaction = Interaction::DragOrbit { new_orbit: 0.0 };
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
                Interaction::DragOrbit { new_orbit } => {
                    self.radius = *new_orbit;
                    self.interaction = Interaction::None;
                },
                Interaction::None => {}
            }
        }

        //always pass release event down
        for c in &mut self.children{
            c.on_drag_end();
        }

    }

    fn on_orbit_handle(&self, loc: Pos2) -> bool{
        let handle_rad = (loc-self.center).length();
        handle_rad > (self.radius - Self::HANDLE_WIDTH) && handle_rad < (self.radius + Self::HANDLE_WIDTH)
    }

    fn on_planet(&self, loc: Pos2) -> bool{
        //TODO currently calculating "left side" on
        let pos = self.obj_pos();

        let rad = (loc-pos).length();
        rad < Self::OBJSIZE
    }

    ///Notifies that cursor is hovering
    fn on_hover(&mut self, at: Pos2){

        //if hovering over our orbit, thicken line
        match (self.on_orbit_handle(at), self.on_planet(at)){
            (false, false) => {
                //reset orbit render width
                self.orbit_width = Self::ORBIT_LINE_WIDTH
            },
            (true, false) => {
                //only on orbit, widen orbit line
                self.orbit_width = Self::ORBIT_LINE_FAT;
            },
            (_, true) => {
                //on planet, preffer over orbit.
                self.orbit_width = Self::ORBIT_LINE_WIDTH;
            }
        }

        for c in &mut self.children{
            c.on_hover(at);
        }
    }
}

pub struct SolarSystem{
    ///size of the sun, also marks "dead" area
    sun_size: f32,

    hover_at: Pos2,

    orbitals: Vec<Orbital>,

}

impl SolarSystem{
    fn new() -> Self{
        SolarSystem{
            sun_size: 10.0,
            hover_at: Pos2::ZERO,
            orbitals: Vec::new(),
        }
    }
}

pub struct Renderer{
    pub params: Arc<OrbitalParams>,
    pub system: SolarSystem,
}

impl Widget for &mut Renderer{
    fn ui(self, ui: &mut nih_plug_egui::egui::Ui) -> nih_plug_egui::egui::Response {
        let rect = ui.clip_rect();
        let (response, painter) = ui.allocate_painter(rect.size(), Sense::click_and_drag());

        //update hover if there is any
        if let Some(hp) = response.hover_pos(){
            for orb in &mut self.system.orbitals{
                orb.on_hover(hp);
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
                    orbital.on_drag(interaction_pos);
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
        }



        self.paint(rect.center(), &painter);

        response
    }
}

impl Renderer{
    pub fn new(params: Arc<OrbitalParams>) -> Self{
        Renderer {
            params,
            system: SolarSystem::new()
        }
    }

    pub fn paint(&self, center: Pos2, painter: &Painter){
        painter.add(Shape::Circle(CircleShape {
            center,
            radius: self.system.sun_size,
            fill: Color32::LIGHT_YELLOW,
            stroke: Stroke::none()
        }));

        for orbital in self.system.orbitals.iter(){
            orbital.paint(painter);
        }

    }
}
