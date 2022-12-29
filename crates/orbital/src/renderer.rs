use std::{sync::Arc, time::Instant};

use crate::{
    com::{ComMsg, GainType},
    envelope::EnvelopeParams,
    osc::ModulationType,
    OrbitalParams,
};
use crossbeam::channel::Sender;
use nih_plug_egui::egui::{Sense, Widget};

use self::{
    adsrgui::{GainSwitch, Knob},
    modswitch::ModSwitch,
    ppbutton::PPButton,
    solar_system::SolarSystem, switch::Switch,
};

pub mod adsrgui;
pub mod modswitch;
pub mod orbital;
pub mod ppbutton;
pub mod solar_system;
pub mod switch;

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
            .unwrap_or(ModulationType::default());

        let mut local_env: EnvelopeParams = self
            .params
            .adsr
            .lock()
            .map(|m| m.clone())
            .unwrap_or(EnvelopeParams::default());
        let mut env_changed = false;

        let mut gain_ty = self
            .params
            .gain_ty
            .lock()
            .map(|g| g.clone())
            .unwrap_or(GainType::default());

        let mut reset_phase = self
            .params
            .reset_phase
            .lock()
            .map(|g| g.clone())
            .unwrap_or(false);

        let tp = egui::TopBottomPanel::top("Toppanel").show(ui.ctx(), |ui| {
            ui.horizontal_centered(|ui| {
                ui.add(PPButton::new(&mut self.system.paused));
                if ui.add(ModSwitch::new(&mut mod_ty)).changed() {
                    let _ = self
                        .msg_sender
                        .send(ComMsg::ModRelationChanged(mod_ty.clone()));
                }

                if ui.add(GainSwitch::new(&mut gain_ty)).changed() {
                    let _ = self.msg_sender.send(ComMsg::GainChange(gain_ty));
                }

                ui.vertical(|ui| {
                    if ui
                        .add(Knob::new(&mut local_env.delay, 0.0, 1.0).with_label("Delay"))
                        .changed()
                    {
                        env_changed = true;
                    }
                });
                ui.vertical(|ui| {
                    if ui
                        .add(Knob::new(&mut local_env.attack, 0.0, 1.0).with_label("Attack"))
                        .changed()
                    {
                        env_changed = true;
                    }
                });
                ui.vertical(|ui| {
                    if ui
                        .add(Knob::new(&mut local_env.hold, 0.0, 1.0).with_label("Hold"))
                        .changed()
                    {
                        env_changed = true;
                    }
                });
                ui.vertical(|ui| {
                    if ui
                        .add(Knob::new(&mut local_env.decay, 0.0, 1.0).with_label("Decay"))
                        .changed()
                    {
                        env_changed = true;
                    }
                });
                ui.vertical(|ui| {
                    if ui
                        .add(
                            Knob::new(&mut local_env.sustain_level, 0.0, 1.0).with_label("Sustain"),
                        )
                        .changed()
                    {
                        env_changed = true;
                    }
                });
                ui.vertical(|ui| {
                    if ui
                        .add(Knob::new(&mut local_env.release, 0.0, 1.0).with_label("Release"))
                        .changed()
                    {
                        env_changed = true;
                    }
                });

                ui.vertical_centered(|ui|{

                    if ui.add(Switch::new(&mut reset_phase).with_label("Reset Phase")).changed(){
                        let _ = self.msg_sender.send(ComMsg::ResetPhaseChanged(reset_phase));
                    }
                })

            })
        });

        if env_changed {
            let _ = self.msg_sender.send(ComMsg::EnvChanged(local_env));
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
