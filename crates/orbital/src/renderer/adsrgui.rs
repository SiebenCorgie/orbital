use std::{fmt::Display, f32::consts::PI};

use egui::{Widget, Slider, Response, Painter, Sense, plot::PlotPoint, Pos2, Stroke, Color32, Vec2, Rect, epaint::CircleShape, Align2, FontId};
use nih_plug::nih_dbg;

use crate::envelope::EnvelopeParams;

use super::orbital::{TWOPI, rotate_vec2};




pub struct AdsrGui{
    pub params: EnvelopeParams,
}

impl AdsrGui{
    const SIZE: Vec2 = Vec2{x: 120.0, y: 80.0};
}

impl AdsrGui{
    fn draw(&mut self, painter: &Painter){
        let per_reg_len = painter.clip_rect().size().x / 5.0;
        let rect = painter.clip_rect();
        let ltr = |pt| rect.min + pt;


    }
}

impl Widget for &mut AdsrGui{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let(reps, painter) = ui.allocate_painter(AdsrGui::SIZE, Sense::click_and_drag());

        let rect = painter.clip_rect();

        self.draw(&painter);

        //draw lines
        println!("Rect: {:?}", rect);
        let to_px = |loc: Pos2| rect.min + loc.to_vec2();



        painter.line_segment([rect.left_top(), rect.left_bottom()], Stroke::new(1.0, Color32::WHITE));
        painter.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0, Color32::WHITE));



        reps
    }
}


pub struct Knob<'a, T>{
    pub value: &'a mut T,
    //rectangel
    pub size: f32,
    pub min: T,
    pub max: T
}

impl<'a, T> Knob<'a, T>{
    pub fn new(value: &'a mut T, min: T, max: T) -> Self{
        Knob {
            value,
            size: 50.0,
            min,
            max
        }
    }

    pub fn with_size(mut self, size: f32) -> Self{
        self.size = size;
        self
    }

    fn offset(&self) -> f32{
        (self.size / 2.0) - 5.0
    }
}

impl<'a> Knob<'a, f32> {
    fn angle_to_value(&self, angle: f32) -> f32{
        let perc = angle / TWOPI;
        self.min + ((self.max - self.min) * perc)
    }

    fn value_to_angle(&self, val: f32) -> f32{
        let perc = (val - self.min) / (self.max - self.min);
        (TWOPI * perc).clamp(0.0, TWOPI)
    }
}

impl<'a> Widget for Knob<'a, f32>{
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let (mut resp, painter) = ui.allocate_painter(Vec2::splat(self.size), Sense::click_and_drag());
        let rect = painter.clip_rect();
        let knob_offset = self.offset();

        //find the location and update the value
        if resp.dragged(){
            if let Some(at) = ui.input().pointer.interact_pos(){
                let at_prime = at - rect.center();
                let angle = (Vec2::Y.dot(at_prime)
                             / (at_prime.length() * 1.0))
                    .acos();

                let angle = if at.x < rect.center().x {
                    //TWOPI - angle
                    angle
                } else {
                    TWOPI - angle
                };

                *self.value = self.angle_to_value(angle);
                resp.mark_changed();
            }
        }

        if resp.clicked(){
            *self.value = self.min;
            resp.mark_changed();
        }

        painter.circle(rect.center(), knob_offset, Color32::TRANSPARENT, Stroke::new(1.0, Color32::LIGHT_GRAY));
        let at = rotate_vec2(Vec2::Y * knob_offset, self.value_to_angle(*self.value));
        painter.circle(rect.center() + at, 2.0, Color32::WHITE, Stroke::none());
        painter.line_segment([rect.center_bottom(), rect.center_bottom() - Vec2{x: 0.0, y: 10.0}], Stroke::new(1.0, Color32::WHITE));
        painter.text(rect.center(), Align2::CENTER_CENTER, format!("{:.2}", self.value), FontId::default(), Color32::WHITE);
        resp
    }
}

impl<'a> Knob<'a, f64> {
    fn angle_to_value(&self, angle: f32) -> f64{
        let perc = angle / TWOPI;
        self.min + ((self.max - self.min) * perc as f64)
    }

    fn value_to_angle(&self, val: f64) -> f32{
        let perc = ((val - self.min) / (self.max - self.min)) as f32;
        (TWOPI * perc).clamp(0.0, TWOPI)
    }
}

impl<'a> Widget for Knob<'a, f64>{
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let (mut resp, painter) = ui.allocate_painter(Vec2::splat(self.size), Sense::click_and_drag());
        let rect = painter.clip_rect();
        let knob_offset = self.offset();

        //find the location and update the value
        if resp.dragged(){
            if let Some(at) = ui.input().pointer.interact_pos(){
                let at_prime = at - rect.center();
                let angle = (Vec2::Y.dot(at_prime)
                             / (at_prime.length() * 1.0))
                    .acos();

                let angle = if at.x < rect.center().x {
                    //TWOPI - angle
                    angle
                } else {
                    TWOPI - angle
                };

                *self.value = self.angle_to_value(angle);
                resp.mark_changed();
            }
        }

        if resp.clicked(){
            *self.value = self.min;
            resp.mark_changed();
        }

        painter.circle(rect.center(), knob_offset, Color32::TRANSPARENT, Stroke::new(1.0, Color32::LIGHT_GRAY));
        let at = rotate_vec2(Vec2::Y * knob_offset, self.value_to_angle(*self.value));
        painter.circle(rect.center() + at, 2.0, Color32::WHITE, Stroke::none());
        painter.line_segment([rect.center_bottom(), rect.center_bottom() - Vec2{x: 0.0, y: 10.0}], Stroke::new(1.0, Color32::WHITE));
        painter.text(rect.center(), Align2::CENTER_CENTER, format!("{:.2}", self.value), FontId::default(), Color32::WHITE);
        resp
    }
}
