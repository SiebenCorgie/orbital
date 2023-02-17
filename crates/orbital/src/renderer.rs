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
    solar_system::SolarSystem,
    switch::Switch,
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
    show_help: bool,
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

        let tp = egui::TopBottomPanel::top("Toppanel")
            .max_height(50.0)
            .resizable(false)
            .min_height(10.0)
            .show(ui.ctx(), |ui| {
                ui.centered_and_justified(|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.spacing();
                            if ui.link("Help").clicked() {
                                self.show_help = !self.show_help;
                            }
                            if ui.link("Creator").clicked() {
                                let _ = open::that("https://siebencorgie.rs");
                            }
                            if ui.link("Donate").clicked() {
                                let _ = open::that("https://ko-fi.com/siebencorgie");
                            }
                            if ui.link("GitHub").clicked() {
                                let _ = open::that("https://github.com/SiebenCorgie/orbital");
                            }
                        });

                        ui.add_space(10.0);

                        //ui.add(PPButton::new(&mut self.system.paused));
                        if ui.add(ModSwitch::new(&mut mod_ty)).changed() {
                            let _ = self
                                .msg_sender
                                .send(ComMsg::ModRelationChanged(mod_ty.clone()));
                        }

                        if ui.add(GainSwitch::new(&mut gain_ty)).changed() {
                            let _ = self.msg_sender.send(ComMsg::GainChange(gain_ty));
                        }

                        ui.add_space(10.0);

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
                                .add(
                                    Knob::new(&mut local_env.attack, 0.0, 1.0).with_label("Attack"),
                                )
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
                                    Knob::new(&mut local_env.sustain_level, 0.0, 1.0)
                                        .with_label("Sustain"),
                                )
                                .changed()
                            {
                                env_changed = true;
                            }
                        });
                        ui.vertical(|ui| {
                            if ui
                                .add(
                                    Knob::new(&mut local_env.release, 0.0, 1.0)
                                        .with_label("Release"),
                                )
                                .changed()
                            {
                                env_changed = true;
                            }
                        });

                        ui.add_space(10.0);

                        ui.vertical(|ui| {
                            if ui
                                .add(Switch::new(&mut reset_phase).with_label("Reset Phase"))
                                .changed()
                            {
                                let _ =
                                    self.msg_sender.send(ComMsg::ResetPhaseChanged(reset_phase));
                            }
                        });
                        ui.add_space(20.0);
                        ui.vertical(|ui| {
                            ui.add_space(10.0);
                            if ui.add(PPButton::new(&mut self.system.is_paused)).clicked() {
                                self.system.reset_anim_state();
                            }
                        })
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

        if self.show_help {
            let _bottom_resp = egui::panel::TopBottomPanel::bottom("bottom_panel")
                //.max_height(20.0)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    //ui.centered_and_justified(|ui| {
                        ui.label("
There are four main interactions. :
    1.: Click somewhere to create a new orbital, right click an orbital to delete it.
    2.: Select and drag an existing orbit to adjust the influence of the orbital on its parent.
    3.: Drag out a sibling orbital from an existing one by clicking and dragging out the edge.
    4.: Scroll while hovering over an object to adjust its relative or absolute speed depending on the selected mode on the top bar.");
                    //})
                });
        }

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
            show_help: false,
            system,
        }
    }
}
