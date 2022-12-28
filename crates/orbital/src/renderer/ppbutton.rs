use egui::{Sense, Stroke, Vec2, Widget};

use super::adsrgui::GainSwitch;

pub struct PPButton<'a> {
    state: &'a mut bool,
}
impl<'a> PPButton<'a> {
    const SIZE: f32 = 50.0;
    const REDUCE: f32 = 20.0;
    const PAUSE_WIDTH: f32 = 5.0;
    const ICOSIZE: f32 = Self::SIZE - Self::REDUCE;
    const STROKE: Stroke = GainSwitch::STROKE;
    pub fn new(state: &'a mut bool) -> Self {
        Self { state }
    }
}

impl<'a> Widget for PPButton<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (mut resp, painter) = ui.allocate_painter(Vec2::splat(Self::SIZE), Sense::click());

        if resp.clicked() {
            *self.state = !*self.state;
            resp.mark_changed();
        }

        let rect = painter.clip_rect();
        match self.state {
            true => {
                //draw line for play
                painter.line_segment(
                    [
                        rect.center() - Vec2::splat(Self::ICOSIZE / 2.0),
                        rect.center()
                            + Vec2 {
                                x: -Self::ICOSIZE / 2.0,
                                y: Self::ICOSIZE / 2.0,
                            },
                    ],
                    Self::STROKE,
                );
                painter.line_segment(
                    [
                        rect.center()
                            + Vec2 {
                                x: -Self::ICOSIZE / 2.0,
                                y: Self::ICOSIZE / 2.0,
                            },
                        rect.center()
                            + Vec2 {
                                x: Self::ICOSIZE / 2.0,
                                y: 0.0,
                            },
                    ],
                    Self::STROKE,
                );
                painter.line_segment(
                    [
                        rect.center() - Vec2::splat(Self::ICOSIZE / 2.0),
                        rect.center()
                            + Vec2 {
                                x: Self::ICOSIZE / 2.0,
                                y: 0.0,
                            },
                    ],
                    Self::STROKE,
                );
            }
            false => {
                //draw line for pause

                painter.line_segment(
                    [
                        rect.center()
                            - Vec2 {
                                x: -Self::PAUSE_WIDTH,
                                y: -Self::ICOSIZE / 2.0,
                            },
                        rect.center()
                            - Vec2 {
                                x: -Self::PAUSE_WIDTH,
                                y: Self::ICOSIZE / 2.0,
                            },
                    ],
                    Self::STROKE,
                );
                painter.line_segment(
                    [
                        rect.center()
                            - Vec2 {
                                x: Self::PAUSE_WIDTH,
                                y: -Self::ICOSIZE / 2.0,
                            },
                        rect.center()
                            - Vec2 {
                                x: Self::PAUSE_WIDTH,
                                y: Self::ICOSIZE / 2.0,
                            },
                    ],
                    Self::STROKE,
                );
            }
        }

        resp
    }
}
