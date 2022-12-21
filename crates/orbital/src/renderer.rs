use std::{sync::Arc, time::Instant};

use crossbeam::channel::Sender;
use egui::{Response, ComboBox};
use nih_plug_egui::egui::{Widget, Sense};
use crate::{OrbitalParams, com::ComMsg, osc::ModulationType};

use self::solar_system::SolarSystem;


pub mod solar_system;
pub mod orbital;

pub struct Renderer{
    pub params: Arc<OrbitalParams>,
    pub system: SolarSystem,
    pub last_update: Instant,
    pub msg_sender: Sender<ComMsg>,
}

impl Widget for &mut Renderer{
    fn ui(self, ui: &mut nih_plug_egui::egui::Ui) -> nih_plug_egui::egui::Response {

        let mut mod_ty = self.params.mod_ty.lock().map(|t|t.clone()).unwrap_or(ModulationType::Absolute);


        let tp = egui::TopBottomPanel::top("Toppanel").show(ui.ctx(), |ui|{
            ui.horizontal(|ui|{
                if ui.button("Pause").clicked(){
                    self.system.paused = !self.system.paused;
                }
                ui.vertical(|ui|{
                    egui::ComboBox::from_id_source("modty")
                        .selected_text(format!("{:?}", mod_ty))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut mod_ty, ModulationType::Absolute, "Absolute");
                            ui.selectable_value(&mut mod_ty, ModulationType::Relative, "Relative");
                        });
                });
            })
        });
        let ctpanel = egui::CentralPanel::default().show(ui.ctx(), |ui| {
            let rect = ui.clip_rect();
            let (response, painter) = ui.allocate_painter(rect.size(), Sense::click_and_drag());
            self.system.handle_response(&mut self.msg_sender, &response, &ui.input());

            self.system.paint(rect.center(), &painter);
        });

        if let Ok(mut val) = self.params.mod_ty.try_lock() {
            *val = mod_ty;
        }

        tp.response.union(ctpanel.response)
    }
}

impl Renderer{
    pub fn new(params: Arc<OrbitalParams>, com_sender: Sender<ComMsg>) -> Self{
        let system = params.solar_system.lock().map(|s| s.clone()).unwrap_or(SolarSystem::new());
        Renderer {
            params,
            last_update: Instant::now(),
            msg_sender: com_sender,
            system,
        }
    }
}
