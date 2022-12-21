use std::{sync::Arc, time::Instant};

use crossbeam::channel::Sender;
use nih_plug_egui::egui::{Widget, Sense};
use crate::{OrbitalParams, com::ComMsg};

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
        let rect = ui.clip_rect();
        let (response, painter) = ui.allocate_painter(rect.size(), Sense::click_and_drag());

        self.system.handle_response(&mut self.msg_sender, &response, &ui.input());

        self.system.paint(rect.center(), &painter);

        response
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
