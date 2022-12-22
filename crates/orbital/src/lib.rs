use atomic_float::AtomicF32;
use com::ComMsg;
use crossbeam::channel::{Sender, Receiver, TryRecvError};
use nih_plug::{prelude::*, params::persist, util::midi_note_to_freq};
use nih_plug_egui::{create_egui_editor, egui::{self, Painter}, EguiState};
use osc::{OscillatorBank, ModulationType};
use osc_array::OscArray;
use renderer::{Renderer, solar_system::SolarSystem};
use std::sync::{Arc, Mutex};


mod renderer;
mod osc;
mod osc_array;
mod com;
mod envelope;

/// The time it takes for the peak meter to decay by 12 dB after switching to complete silence.
const PEAK_METER_DECAY_MS: f64 = 150.0;

pub type Time = f64;

/// This is mostly identical to the gain example, minus some fluff, and with a GUI.
pub struct Orbital {
    params: Arc<OrbitalParams>,

    com_channel: (Sender<ComMsg>, Receiver<ComMsg>),
    ///in audio-thread osc bank
    synth: OscArray,

    ///last known time (in sec.)
    transport_time: Time,
}



#[derive(Params)]
pub struct OrbitalParams {
    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,
    #[persist = "modty"]
    pub mod_ty: Arc<Mutex<ModulationType>>,
    #[persist = "Synth"]
    pub synth: Arc<Mutex<OscArray>>,
    #[persist = "SolarSystem"]
    pub solar_system: Arc<Mutex<SolarSystem>>,
}

impl Default for Orbital {
    fn default() -> Self {
        Self {
            params: Arc::new(OrbitalParams::default()),
            com_channel: crossbeam::channel::unbounded(),
            synth: OscArray::default(),
            transport_time: 0.0,
        }
    }
}

impl Default for OrbitalParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(800, 800),

            // See the main gain example for more details
            mod_ty: Arc::new(Mutex::new(ModulationType::Absolute)),
            synth: Arc::new(Mutex::new(OscArray::default())),
            solar_system: Arc::new(Mutex::new(SolarSystem::new())),
        }
    }
}

impl Plugin for Orbital {
    const NAME: &'static str = "Orbital";
    const VENDOR: &'static str = "Tendsin's Lab";
    const URL: &'static str = "https://siebencorgie.rs";
    const EMAIL: &'static str = "info@example.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const DEFAULT_INPUT_CHANNELS: u32 = 0;
    const DEFAULT_OUTPUT_CHANNELS: u32 = 2;
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let renderer = Renderer::new(params, self.com_channel.0.clone());
        create_egui_editor(
            self.params.editor_state.clone(),
            renderer,
            |_, _| {},
            move |egui_ctx, _setter, renderer| {
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    ui.add(renderer);
                });
            },
        )
    }

    fn accepts_bus_config(&self, config: &BusConfig) -> bool {
        // This works with any symmetrical IO layout
        config.num_input_channels == 0 && config.num_output_channels == 2
    }

    fn initialize(
        &mut self,
        _bus_config: &BusConfig,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        nih_log!("Init");
        true
    }


    fn deactivate(&mut self) {

        self.transport_time = 0.0;
        //feed back current parameter state
        if let Ok(mut lck) = self.params.synth.lock(){
            *lck = self.synth.clone();
        }else{
            nih_error!("Failed to serialize osc bank");
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {

        /*
        //advance time stamp either from daw time, or by counting buffer samples
        if let Some(stamp) = context.transport().pos_seconds(){
            self.transport_time = stamp as f32;
        }else{
            //calculate based on buffer size and sample rate
            self.transport_time += (buffer.len() / buffer.channels()) as f32  / context.transport().sample_rate;
        }*/

        let buffer_length = (buffer.len() / buffer.channels()) as Time / context.transport().sample_rate as f64;

        //try at most 10
        // TODO: check if we maybe should do that async
        for _try in 0..10{
            match self.com_channel.1.try_recv(){
                Ok(msg) => {
                    self.synth.bank.on_msg(msg);
                }
                Err(e) => {
                    match e {
                        TryRecvError::Disconnected => {
                            nih_log!("com was disconnected!");
                        },
                        TryRecvError::Empty => break, //end recy loop for now
                    }
                }
            }
        }

        if let Ok(ty) = self.params.mod_ty.try_lock(){
            self.synth.bank.mod_ty = ty.clone();
        }

        while let Some(ev) = context.next_event(){
            match ev{
                NoteEvent::NoteOn { note, .. } => self.synth.note_on(note, self.transport_time),
                NoteEvent::NoteOff { note, .. } => self.synth.note_off(note, self.transport_time),
                _ => {}
            }
        }


        self.synth.process(buffer, context.transport().sample_rate, self.transport_time, buffer_length);
        //update time
        self.transport_time += buffer_length;

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Orbital {
    const CLAP_ID: &'static str = "com.tendsins-lab.orbital";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Cosmic FM-Synth");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Stereo,
        ClapFeature::Utility,
        ClapFeature::Synthesizer,
    ];
}

impl Vst3Plugin for Orbital {
    const VST3_CLASS_ID: [u8; 16] = *b"OrbitalSynthnnns";
    const VST3_CATEGORIES: &'static str = "Instrument|Synth";
}

nih_export_clap!(Orbital);
nih_export_vst3!(Orbital);
