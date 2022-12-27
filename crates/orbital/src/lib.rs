use com::{ComMsg, GainType};
use crossbeam::channel::{Receiver, Sender, TryRecvError};
use envelope::EnvelopeParams;
use nih_plug::{
    nih_error, nih_export_clap, nih_export_vst3, nih_log,
    prelude::{
        AsyncExecutor, AuxiliaryBuffers, Buffer, BufferConfig, BusConfig, ClapFeature, ClapPlugin,
        Editor, InitContext, MidiConfig, NoteEvent, Params, Plugin, ProcessContext, ProcessStatus,
        Vst3Plugin,
    },
};
use nih_plug_egui::{create_egui_editor, EguiState};
use osc::ModulationType;
use osc_array::OscArray;
use renderer::{solar_system::SolarSystem, Renderer};
use std::sync::{Arc, Mutex};

mod com;
mod envelope;
mod osc;
mod osc_array;
mod renderer;

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
    #[persist = "adsr_save"]
    pub adsr: Arc<Mutex<EnvelopeParams>>,

    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,
    #[persist = "modty"]
    pub mod_ty: Arc<Mutex<ModulationType>>,
    #[persist = "gainty"]
    pub gain_ty: Arc<Mutex<GainType>>,
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
            adsr: Arc::new(Mutex::new(EnvelopeParams::default())),
            // See the main gain example for more details
            mod_ty: Arc::new(Mutex::new(ModulationType::Absolute)),
            gain_ty: Arc::new(Mutex::new(GainType::Linear)),
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
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        nih_log!("Init");

        //init synth to current state, or default
        self.synth.bank.on_state_change(
            self.params
                .solar_system
                .lock()
                .map(|lck| lck.get_solar_state())
                .unwrap_or(SolarSystem::new().get_solar_state()),
        );
        self.synth.set_envelopes(
            self.params
                .adsr
                .lock()
                .map(|i| i.clone())
                .unwrap_or(EnvelopeParams::default()),
        );
        self.synth.bank.mod_ty = self
            .params
            .mod_ty
            .lock()
            .map(|m| m.clone())
            .unwrap_or(ModulationType::Absolute);
        true
    }

    fn deactivate(&mut self) {
        self.transport_time = 0.0;
        //feed back current parameter state
        if let Ok(mut lck) = self.params.synth.lock() {
            *lck = self.synth.clone();
        } else {
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

        let buffer_length = buffer.len() as Time / context.transport().sample_rate as f64;
        let sample_time = 1.0 / context.transport().sample_rate as Time;

        //try at most 10
        // TODO: check if we maybe should do that async
        for _try in 0..10 {
            match self.com_channel.1.try_recv() {
                Ok(msg) => {
                    match msg {
                        ComMsg::SolarState(s) => self.synth.bank.on_state_change(s),
                        ComMsg::ModRelationChanged(new) => {
                            if let Ok(mut mr) = self.params.mod_ty.try_lock() {
                                *mr = new.clone();
                            }
                            self.synth.bank.mod_ty = new
                        }
                        //Note we only use the notify
                        ComMsg::EnvChanged(env_param) => {
                            if let Ok(mut p) = self.params.adsr.try_lock() {
                                *p = env_param.clone();
                            }
                            self.synth.set_envelopes(env_param)
                        },
                        ComMsg::GainChange(new_gain) => {
                            if let Ok(mut p) = self.params.gain_ty.try_lock(){
                                *p = new_gain.clone();
                            }
                            self.synth.bank.gain_ty = new_gain;
                        }
                    }
                }
                Err(e) => {
                    match e {
                        TryRecvError::Disconnected => {
                            nih_log!("com was disconnected!");
                        }
                        TryRecvError::Empty => break, //end recy loop for now
                    }
                }
            }
        }

        while let Some(ev) = context.next_event() {
            match ev {
                NoteEvent::NoteOn { note, timing, .. } => self
                    .synth
                    .note_on(note, self.transport_time + timing as Time * sample_time),
                NoteEvent::NoteOff { note, timing, .. } => self
                    .synth
                    .note_off(note, self.transport_time + timing as Time * sample_time),
                _ => {}
            }
        }

        self.synth
            .process(buffer, context.transport().sample_rate, self.transport_time);
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
