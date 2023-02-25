use super::orbital::{rotate_vec2, TWOPI};
use crate::com::GainType;
use egui::{
    epaint::CubicBezierShape, Align2, Color32, FontId, Label, Response, Sense, Shape, Stroke, Vec2,
    Widget,
};
use nih_plug::prelude::{Param, ParamSetter};

#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Knob<'a, P: Param> {
    param: &'a P,
    setter: &'a ParamSetter<'a>,
    //rect
    pub size: f32,
    pub label: Option<&'a str>,
}

impl<'a, P: Param> Knob<'a, P> {
    pub fn new(param: &'a P, setter: &'a ParamSetter<'a>) -> Self {
        Knob {
            param,
            setter,
            size: 50.0,
            label: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    fn offset(&self) -> f32 {
        (self.size / 2.0) - 5.0
    }
}

impl<'a, P: Param> Knob<'a, P> {
    fn angle_to_normalized_value(&self, angle: f32) -> f32 {
        angle / TWOPI
    }

    fn value_to_angle(&self, normalized: f32) -> f32 {
        (TWOPI * normalized).clamp(0.0, TWOPI)
    }

    fn plain_value(&self) -> P::Plain {
        self.param.modulated_plain_value()
    }

    fn set_normalized_value(&self, normalized: f32) {
        // This snaps to the nearest plain value if the parameter is stepped in some way.
        // TODO: As an optimization, we could add a `const CONTINUOUS: bool` to the parameter to
        //       avoid this normalized->plain->normalized conversion for parameters that don't need
        //       it
        let value = self.param.preview_plain(normalized);
        if value != self.plain_value() {
            self.setter.set_parameter(self.param, value);
        }
    }
}

impl<'a, P: Param> Widget for Knob<'a, P> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let (mut resp, painter) =
            ui.allocate_painter(Vec2::splat(self.size), Sense::click_and_drag());
        let rect = painter.clip_rect();
        let knob_offset = self.offset();

        let stroke_width = if resp.hovered() { 2.0 } else { 1.0 };

        //find the location and update the value
        if resp.dragged() {
            if let Some(at) = ui.input().pointer.interact_pos() {
                let at_prime = at - rect.center();
                let angle = (Vec2::Y.dot(at_prime) / (at_prime.length() * 1.0)).acos();

                let angle = if at.x < rect.center().x {
                    //TWOPI - angle
                    angle
                } else {
                    TWOPI - angle
                };

                self.set_normalized_value(self.angle_to_normalized_value(angle));
                resp.mark_changed();
            }
        }

        if resp.clicked() {
            if ui
                .input()
                .pointer
                .button_double_clicked(egui::PointerButton::Primary)
            {
                //on double click, reset value
                self.set_normalized_value(self.param.default_normalized_value());
                resp.mark_changed();
            }
        }

        painter.circle(
            rect.center(),
            knob_offset,
            Color32::TRANSPARENT,
            Stroke::new(stroke_width, Color32::LIGHT_GRAY),
        );

        let at = rotate_vec2(
            Vec2::Y * knob_offset,
            self.value_to_angle(self.param.modulated_normalized_value()),
        );
        painter.circle(rect.center() + at, 2.0, Color32::WHITE, Stroke::none());
        painter.line_segment(
            [
                rect.center_bottom(),
                rect.center_bottom() - Vec2 { x: 0.0, y: 10.0 },
            ],
            Stroke::new(stroke_width, Color32::WHITE),
        );
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            format!(
                "{}",
                self.param
                    .normalized_value_to_string(self.param.modulated_normalized_value(), true)
            ),
            FontId::default(),
            Color32::WHITE,
        );

        if let Some(label) = self.label {
            ui.add_sized(
                Vec2 {
                    x: self.size,
                    y: ui.available_height(),
                },
                Label::new(label),
            );
        }
        resp
    }
}

pub struct GainSwitch<'a> {
    value: &'a mut GainType,
}

impl<'a> GainSwitch<'a> {
    pub const SIZE: Vec2 = Vec2 { x: 100.0, y: 65.0 };
    const XOFF: f32 = 20.0;
    const YOFF: f32 = 15.0;
    pub const COLOR: Color32 = Color32::WHITE;
    pub const STROKE: Stroke = Stroke {
        width: 1.0,
        color: Self::COLOR,
    };
    pub fn new(value: &'a mut GainType) -> Self {
        GainSwitch { value }
    }
}

impl<'a> Widget for GainSwitch<'a> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let (mut resp, painter) = ui.allocate_painter(Self::SIZE, Sense::click());

        if resp.clicked() {
            self.value.next();
            resp.mark_changed();
        }

        let rect = painter.clip_rect();

        let stroke = if resp.hovered() {
            let mut s = Self::STROKE;
            s.width = 2.0;
            s
        } else {
            Self::STROKE
        };

        match self.value {
            GainType::Linear => {
                painter.line_segment(
                    [
                        rect.left_center()
                            + Vec2 {
                                x: 0.0,
                                y: Self::YOFF,
                            },
                        rect.center()
                            + Vec2 {
                                x: -Self::XOFF,
                                y: Self::YOFF,
                            },
                    ],
                    stroke,
                );

                painter.line_segment(
                    [
                        rect.center()
                            + Vec2 {
                                x: -Self::XOFF,
                                y: Self::YOFF,
                            },
                        rect.center()
                            + Vec2 {
                                x: Self::XOFF,
                                y: -Self::YOFF,
                            },
                    ],
                    stroke,
                );

                painter.line_segment(
                    [
                        rect.center()
                            + Vec2 {
                                x: Self::XOFF,
                                y: -Self::YOFF,
                            },
                        rect.right_center()
                            + Vec2 {
                                x: 0.0,
                                y: -Self::YOFF,
                            },
                    ],
                    stroke,
                );

                painter.text(
                    rect.center_bottom(),
                    Align2::CENTER_BOTTOM,
                    "Linear",
                    FontId::default(),
                    Color32::GRAY,
                );
            }
            GainType::Sigmoid => {
                painter.line_segment(
                    [
                        rect.left_center()
                            + Vec2 {
                                x: 0.0,
                                y: Self::YOFF,
                            },
                        rect.center()
                            + Vec2 {
                                x: -Self::XOFF,
                                y: Self::YOFF,
                            },
                    ],
                    stroke,
                );

                painter.add(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                    [
                        rect.center()
                            + Vec2 {
                                x: -Self::XOFF,
                                y: Self::YOFF,
                            },
                        rect.center()
                            + Vec2 {
                                x: 0.0,
                                y: Self::YOFF,
                            },
                        rect.center()
                            + Vec2 {
                                x: 0.0,
                                y: -Self::YOFF,
                            },
                        rect.center()
                            + Vec2 {
                                x: Self::XOFF,
                                y: -Self::YOFF,
                            },
                    ],
                    false,
                    Color32::TRANSPARENT,
                    stroke,
                )));

                painter.line_segment(
                    [
                        rect.center()
                            + Vec2 {
                                x: Self::XOFF,
                                y: -Self::YOFF,
                            },
                        rect.right_center()
                            + Vec2 {
                                x: 0.0,
                                y: -Self::YOFF,
                            },
                    ],
                    stroke,
                );

                painter.text(
                    rect.center_bottom(),
                    Align2::CENTER_BOTTOM,
                    "Sigmoid",
                    FontId::default(),
                    Color32::GRAY,
                );
            }
        }
        resp
    }
}
