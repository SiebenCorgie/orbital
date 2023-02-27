use std::{sync::Arc, time::Instant};

use crate::{
    com::{ComMsg, GainType},
    osc::ModulationType,
    OrbitalParams,
};
use crossbeam::channel::Sender;
use egui::Context;
use nih_plug::{nih_error, prelude::ParamSetter};
use nih_plug_egui::egui::Sense;

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
    pub last_update: Instant,
    pub msg_sender: Sender<ComMsg>,
    show_help: bool,
}

impl Renderer {
    pub fn draw(&mut self, eguictx: &Context, setter: &ParamSetter) {
        //setup egui ui context as you usually would. But we gain the `setter` param which we cant
        // access if we implement `ui()` in egui's Widget trait.
        egui::CentralPanel::default().show(eguictx, |ui|{

        let mut mod_ty = self
            .params
            .mod_ty
            .lock()
            .map(|t| t.clone())
            .unwrap_or(ModulationType::default());

        let mut gain_ty = self
            .params
            .gain_ty
            .lock()
            .map(|g| g.clone())
            .unwrap_or(GainType::default());


        let _tp = egui::TopBottomPanel::top("Toppanel")
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
                            ui.add(Knob::new(&self.params.delay, setter).with_label("Delay"))
                        });
                        ui.vertical(|ui| {
                            ui.add(Knob::new(&self.params.attack, setter).with_label("Attack"))
                        });
                        ui.vertical(|ui| {
                                ui.add(Knob::new(&self.params.hold, setter).with_label("Hold"))
                        });
                        ui.vertical(|ui| {
                                ui.add(Knob::new(&self.params.decay, setter).with_label("Decay"))
                        });
                        ui.vertical(|ui| {
                                ui.add(Knob::new(&self.params.sustain, setter).with_label("Sustain"))
                        });
                        ui.vertical(|ui| {
                                ui.add(Knob::new(&self.params.release, setter).with_label("Release"))
                        });

                        ui.add_space(10.0);

                        ui.vertical(|ui| {
                            ui.add(Switch::new(&self.params.reset_phase, setter).with_label("Reset Phase"))
                        });
                        ui.add_space(20.0);
                        ui.vertical(|ui| {
                            if let Ok(mut system) = self.params.solar_system.try_write(){
                                ui.add_space(10.0);
                                if ui.add(PPButton::new(&mut system.is_paused)).clicked() {
                                    system.reset_anim_state();
                                }
                            }else{
                                nih_error!("Could not set anim state!");
                            }

                        })
                    })
                })
            });

        let _ctpanel = egui::CentralPanel::default().show(ui.ctx(), |ui| {
            let rect = ui.clip_rect();
            let (response, painter) = ui.allocate_painter(rect.size(), Sense::click_and_drag());
            if let Ok(mut system) = self.params.solar_system.try_write(){
                system.handle_response(&mut self.msg_sender, &response, &ui.input());
                system.paint(rect.center(), &painter);
            }else{
                nih_error!("Could not set solar state!");
            }
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

        //tp.response.union(ctpanel.response)
        //
        });
    }
}

impl Renderer {
    pub fn new(params: Arc<OrbitalParams>, com_sender: Sender<ComMsg>) -> Self {
        Renderer {
            params,
            last_update: Instant::now(),
            msg_sender: com_sender,
            show_help: false,
        }
    }
}
