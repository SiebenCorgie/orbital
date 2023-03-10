use egui::{Align2, Color32, FontId, Sense, Stroke, Vec2, Widget};

use crate::osc::ModulationType;

use super::adsrgui::GainSwitch;

pub struct ModSwitch<'a> {
    value: &'a mut ModulationType,
}

impl<'a> ModSwitch<'a> {
    const SIZE: Vec2 = GainSwitch::SIZE;
    const SPLIT: f32 = 10.0;
    const STROKE: Stroke = GainSwitch::STROKE;
    pub fn new(value: &'a mut ModulationType) -> Self {
        ModSwitch { value }
    }
}

impl<'a> Widget for ModSwitch<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (mut resp, painter) = ui.allocate_painter(Self::SIZE, Sense::click());

        let rect = painter.clip_rect();

        if resp.clicked() {
            *self.value = self.value.next();
            resp.mark_changed();
        }
        let stroke = if resp.hovered(){
            let mut s = Self::STROKE;
            s.width = 2.0;
            s
        }else{
            Self::STROKE
        };

        match self.value {
            ModulationType::Absolute => {
                painter.line_segment(
                    [
                        rect.right_center()
                            + Vec2 {
                                x: 0.0,
                                y: -Self::SPLIT,
                            },
                        rect.left_center()
                            + Vec2 {
                                x: 0.0,
                                y: -Self::SPLIT,
                            },
                    ],
                    stroke
                );

                painter.line_segment(
                    [
                        rect.right_center()
                            + Vec2 {
                                x: 0.0,
                                y: Self::SPLIT,
                            },
                        rect.left_center()
                            + Vec2 {
                                x: 0.0,
                                y: Self::SPLIT,
                            },
                    ],
                    stroke
                );
                painter.text(
                    rect.center_bottom(),
                    Align2::CENTER_BOTTOM,
                    "Absolute",
                    FontId::default(),
                    Color32::GRAY,
                );
            }
            ModulationType::Relative => {
                painter.line_segment(
                    [
                        rect.right_center()
                            + Vec2 {
                                x: 0.0,
                                y: -Self::SPLIT,
                            },
                        rect.left_center()
                            + Vec2 {
                                x: 0.0,
                                y: -Self::SPLIT,
                            },
                    ],
                    stroke
                );

                painter.line_segment(
                    [
                        rect.center()
                            + Vec2 {
                                x: 0.0,
                                y: -Self::SPLIT,
                            },
                        rect.center()
                            + Vec2 {
                                x: 0.0,
                                y: Self::SPLIT,
                            },
                    ],
                    stroke
                );

                painter.line_segment(
                    [
                        rect.center()
                            + Vec2 {
                                x: 0.0,
                                y: Self::SPLIT,
                            },
                        rect.right_center()
                            + Vec2 {
                                x: 0.0,
                                y: Self::SPLIT,
                            },
                    ],
                    stroke
                );
                painter.text(
                    rect.center_bottom(),
                    Align2::CENTER_BOTTOM,
                    "Relative",
                    FontId::default(),
                    Color32::GRAY,
                );
            }
        }

        resp
    }
}
