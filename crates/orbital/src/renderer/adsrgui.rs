
use egui::{Widget,Response, Sense, Stroke, Color32, Vec2, epaint::CubicBezierShape, Align2, FontId, Shape};
use crate:: com::GainType;
use super::orbital::{TWOPI, rotate_vec2};


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

    #[allow(dead_code)]
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
            if ui.input().pointer.button_double_clicked(egui::PointerButton::Primary){
                *self.value = self.min;
                resp.mark_changed();
            }

            if ui.input().pointer.button_double_clicked(egui::PointerButton::Secondary){
                *self.value = self.max;
                resp.mark_changed();
            }
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
        if resp.dragged_by(egui::PointerButton::Primary){
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
            if ui.input().pointer.button_double_clicked(egui::PointerButton::Primary){
                *self.value = self.min;
                resp.mark_changed();
            }

            if ui.input().pointer.button_double_clicked(egui::PointerButton::Secondary){
                *self.value = self.max;
                resp.mark_changed();
            }
        }

        painter.circle(rect.center(), knob_offset, Color32::TRANSPARENT, Stroke::new(1.0, Color32::LIGHT_GRAY));
        let at = rotate_vec2(Vec2::Y * knob_offset, self.value_to_angle(*self.value));
        painter.circle(rect.center() + at, 2.0, Color32::WHITE, Stroke::none());
        painter.line_segment([rect.center_bottom(), rect.center_bottom() - Vec2{x: 0.0, y: 10.0}], Stroke::new(1.0, Color32::WHITE));
        painter.text(rect.center(), Align2::CENTER_CENTER, format!("{:.2}", self.value), FontId::default(), Color32::WHITE);
        resp
    }
}


pub struct GainSwitch<'a>{
    value: &'a mut GainType
}

impl<'a> GainSwitch<'a>{
    const SIZE: Vec2 = Vec2{x: 100.0, y: 65.0};
    const XOFF: f32 = 20.0;
    const YOFF: f32 = 15.0;
    pub fn new(value: &'a mut GainType) -> Self{
        GainSwitch { value }
    }
}

impl<'a> Widget for GainSwitch<'a>{
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let (mut resp, painter) = ui.allocate_painter(Self::SIZE, Sense::click());

        if resp.clicked(){
            self.value.next();
            resp.mark_changed();
        }

        let rect = painter.clip_rect();

        match self.value {
            GainType::Linear => {
                painter.line_segment(
                    [
                        rect.left_center() + Vec2{x: 0.0, y: Self::YOFF},
                        rect.center() + Vec2{x: -Self::XOFF, y: Self::YOFF}
                    ],
                    Stroke::new(1.0, Color32::GRAY)
                );

                painter.line_segment(
                    [
                        rect.center() + Vec2{x: -Self::XOFF, y: Self::YOFF},
                        rect.center() + Vec2{x: Self::XOFF, y: -Self::YOFF}
                    ],
                    Stroke::new(1.0, Color32::GRAY)
                );

                painter.line_segment(
                    [
                        rect.center() + Vec2{x: Self::XOFF, y: -Self::YOFF},
                        rect.right_center() + Vec2{x: 0.0, y: -Self::YOFF}
                    ],
                    Stroke::new(1.0, Color32::GRAY)
                );

                painter.text(rect.center_bottom(), Align2::CENTER_BOTTOM, "Linear", FontId::default(), Color32::GRAY);
            },
            GainType::Sigmoid => {

                painter.line_segment(
                    [
                        rect.left_center() + Vec2{x: 0.0, y: Self::YOFF},
                        rect.center() + Vec2{x: -Self::XOFF, y: Self::YOFF}
                    ],
                    Stroke::new(1.0, Color32::GRAY)
                );


                painter.add(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                    [
                        rect.center() + Vec2{x: -Self::XOFF, y: Self::YOFF},
                        rect.center() + Vec2{x: 0.0, y: Self::YOFF},
                        rect.center() + Vec2{x: 0.0, y: -Self::YOFF},
                        rect.center() + Vec2{x: Self::XOFF, y: -Self::YOFF},
                    ],
                    false,
                    Color32::TRANSPARENT,
                    Stroke::new(1.0, Color32::GRAY)
                )));

                painter.line_segment(
                    [
                        rect.center() + Vec2{x: Self::XOFF, y: -Self::YOFF},
                        rect.right_center() + Vec2{x: 0.0, y: -Self::YOFF}
                    ],
                    Stroke::new(1.0, Color32::GRAY)
                );

                painter.text(rect.center_bottom(), Align2::CENTER_BOTTOM, "Sigmoid", FontId::default(), Color32::GRAY);
            }
        }
        resp
    }
}
