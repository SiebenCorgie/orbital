use egui::{Painter, Response, Sense, Vec2, Widget};

///Button that uses `paint` to draw its icon.
pub struct PainterButton<'a> {
    painter_function: &'a dyn Fn(&Painter, &mut Response),
    size: Vec2,
}

impl<'a> PainterButton<'a> {
    pub fn new(paint: &'a dyn Fn(&Painter, &mut Response)) -> Self {
        PainterButton {
            painter_function: paint,
            size: Vec2::new(20.0, 20.0),
        }
    }

    pub fn with_size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }
}

impl<'a> Widget for PainterButton<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (mut resp, painter) = ui.allocate_painter(self.size, Sense::click());

        if resp.clicked() {
            resp.mark_changed();
        }

        (self.painter_function)(&painter, &mut resp);

        resp
    }
}
