use egui::{Widget, Label, Color32, Stroke};



pub struct Switch<'a>{
    pub value: &'a mut bool,
    pub label: Option<&'a str>
}

impl<'a> Switch<'a>{
    pub fn new(value: &'a mut bool) -> Self{
        Switch{
            value,
            label: None
        }
    }

    pub fn with_label(mut self, label: &'a str) -> Self{
        self.label = Some(label);
        self
    }
}

impl<'a> Widget for Switch<'a>{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui|{

            let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
            let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
            if response.clicked() {
                *self.value = !*self.value;
                response.mark_changed();
            }
            response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, *self.value, ""));

            if ui.is_rect_visible(rect) {
                let how_on = ui.ctx().animate_bool(response.id, *self.value);
                let visuals = ui.style().interact_selectable(&response, *self.value);
                let rect = rect.expand(visuals.expansion);
                let radius = 0.5 * rect.height();
                ui.painter()
                  .rect(rect, radius, Color32::TRANSPARENT, Stroke::new(1.0, Color32::WHITE));
                let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
                let center = egui::pos2(circle_x, rect.center().y);
                ui.painter()
                  .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);

                if let Some(l) = self.label{
                    ui.add_sized(desired_size, Label::new(l));
                }
            }

            response
        }).inner
    }
}
