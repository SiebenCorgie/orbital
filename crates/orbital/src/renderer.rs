use std::{sync::Arc, time::Instant};

use crate::{com::ComMsg, envelope::EnvelopeParams, osc::ModulationType, OrbitalParams};
use crossbeam::channel::Sender;
use egui::Slider;
use nih_plug_egui::egui::{Sense, Widget};

use self::{solar_system::SolarSystem, adsrgui::{AdsrGui, Knob}};

pub mod orbital;
pub mod solar_system;
pub mod adsrgui;



pub struct Renderer {
    pub params: Arc<OrbitalParams>,
    pub system: SolarSystem,
    pub last_update: Instant,
    pub msg_sender: Sender<ComMsg>,
}

impl Widget for &mut Renderer {
    fn ui(self, ui: &mut nih_plug_egui::egui::Ui) -> nih_plug_egui::egui::Response {
        let mut mod_ty = self
            .params
            .mod_ty
            .lock()
            .map(|t| t.clone())
            .unwrap_or(ModulationType::Absolute);
        let mut mod_ty_changed = false;

        let mut local_env: EnvelopeParams = self
            .params
            .adsr
            .lock()
            .map(|m| m.clone())
            .unwrap_or(EnvelopeParams::default());
        let mut env_changed = false;

        let tp = egui::TopBottomPanel::top("Toppanel").show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                if ui.button("Pause").clicked() {
                    self.system.paused = !self.system.paused;
                }
                ui.vertical(|ui| {
                    ui.label("Modulation relation");
                    egui::ComboBox::from_id_source("modty")
                        .selected_text(format!("{:?}", mod_ty))
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_value(&mut mod_ty, ModulationType::Absolute, "Absolute")
                                .changed()
                            {
                                mod_ty_changed = true;
                            }
                            if ui
                                .selectable_value(&mut mod_ty, ModulationType::Relative, "Relative")
                                .changed()
                            {
                                mod_ty_changed = true;
                            }
                        })
                });

                ui.vertical(|ui|{
                    if ui.add(Knob::new(&mut local_env.delay, 0.0, 1.0)).changed(){
                        env_changed = true;
                    }
                    ui.label("Delay");
                });
                ui.vertical(|ui|{
                    if ui.add(Knob::new(&mut local_env.attack, 0.0, 1.0)).changed(){
                        env_changed = true;
                    }
                    ui.label("Attack");
                });
                ui.vertical(|ui|{
                    if ui.add(Knob::new(&mut local_env.hold, 0.0, 1.0)).changed(){
                        env_changed = true;
                    }
                    ui.label("Hold");
                });
                ui.vertical(|ui|{
                    if ui.add(Knob::new(&mut local_env.decay, 0.0, 1.0)).changed(){
                        env_changed = true;
                    }
                    ui.label("Decay");
                });
                ui.vertical(|ui|{
                    if ui.add(Knob::new(&mut local_env.sustain_level, 0.0, 1.0)).changed(){
                        env_changed = true;
                    }
                    ui.label("Sustain");
                });
                ui.vertical(|ui|{
                    if ui.add(Knob::new(&mut local_env.release, 0.0, 1.0)).changed(){
                        env_changed = true;
                    }
                    ui.label("Release");
                });
            })
        });

        if env_changed {
            let _ = self.msg_sender.send(ComMsg::EnvChanged(local_env));
        }
        if mod_ty_changed {
            let _ = self
                .msg_sender
                .send(ComMsg::ModRelationChanged(mod_ty.clone()));
        }
        let ctpanel = egui::CentralPanel::default().show(ui.ctx(), |ui| {
            let rect = ui.clip_rect();
            let (response, painter) = ui.allocate_painter(rect.size(), Sense::click_and_drag());
            self.system
                .handle_response(&mut self.msg_sender, &response, &ui.input());

            self.system.paint(rect.center(), &painter);
        });

        tp.response.union(ctpanel.response)
    }
}

impl Renderer {
    pub fn new(params: Arc<OrbitalParams>, com_sender: Sender<ComMsg>) -> Self {
        let system = params
            .solar_system
            .lock()
            .map(|s| s.clone())
            .unwrap_or(SolarSystem::new());
        Renderer {
            params,
            last_update: Instant::now(),
            msg_sender: com_sender,
            system,
        }
    }
}
