#![feature(portable_simd)]

use com::{ComMsg, GainType};
use crossbeam::channel::{Receiver, Sender, TryRecvError};
use envelope::EnvelopeParams;
use nih_plug::{
    nih_error, nih_export_clap, nih_export_vst3, nih_log,
    prelude::{
        AsyncExecutor, AudioIOLayout, AuxiliaryBuffers, BoolParam, Buffer, BufferConfig,
        ClapFeature, ClapPlugin, Editor, FloatParam, FloatRange, InitContext, MidiConfig,
        NoteEvent, Params, Plugin, ProcessContext, ProcessStatus, Vst3Plugin, Vst3SubCategory,
    },
};
use nih_plug_egui::{create_egui_editor, EguiState};
use osc::ModulationType;
use osc_array::OscArray;
use renderer::{solar_system::SolarSystem, Renderer};
use std::{
    num::NonZeroU32,
    sync::{Arc, Mutex, RwLock},
};

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

    #[cfg(feature = "profile")]
    server: Option<puffin_http::Server>,
}

impl Orbital {
    const NUM_CHANNELS: u32 = 2;

    fn get_adsr_settings(&self) -> EnvelopeParams {
        EnvelopeParams {
            delay: self.params.delay.value() as f64,
            attack: self.params.attack.value() as f64,
            hold: self.params.hold.value() as f64,
            decay: self.params.decay.value() as f64,
            sustain_level: self.params.sustain.value(),
            release: self.params.release.value() as f64,
        }
    }
}

#[derive(Params)]
pub struct OrbitalParams {
    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,
    #[id = "reset_phase"]
    pub reset_phase: BoolParam,

    #[persist = "modty"]
    pub mod_ty: Arc<Mutex<ModulationType>>,
    #[persist = "gainty"]
    pub gain_ty: Arc<Mutex<GainType>>,
    #[persist = "Synth"]
    pub synth: Arc<Mutex<OscArray>>,
    #[persist = "SolarSystem"]
    pub solar_system: Arc<RwLock<SolarSystem>>,

    #[id = "Delay"]
    pub delay: FloatParam,
    #[id = "Attack"]
    pub attack: FloatParam,
    #[id = "Hold"]
    pub hold: FloatParam,
    #[id = "Decay"]
    pub decay: FloatParam,
    #[id = "Sustain"]
    pub sustain: FloatParam,
    #[id = "Release"]
    pub release: FloatParam,
}

impl Default for Orbital {
    fn default() -> Self {
        Self {
            params: Arc::new(OrbitalParams::default()),
            com_channel: crossbeam::channel::unbounded(),
            synth: OscArray::default(),
            transport_time: 0.0,
            #[cfg(feature = "profile")]
            server: None,
        }
    }
}

impl Default for OrbitalParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(800, 800),
            // See the main gain example for more details
            mod_ty: Arc::new(Mutex::new(ModulationType::default())),
            reset_phase: BoolParam::new("Reset Phase", true),
            gain_ty: Arc::new(Mutex::new(GainType::default())),
            synth: Arc::new(Mutex::new(OscArray::default())),
            solar_system: Arc::new(RwLock::new(SolarSystem::new())),

            delay: FloatParam::new("Gain", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| format!("{:.2}", v))),
            //Otherwise we get such a pesky *clicking* on attack
            attack: FloatParam::new(
                "Attack",
                0.1,
                FloatRange::Linear {
                    min: 0.0001,
                    max: 1.0,
                },
            )
            .with_value_to_string(Arc::new(|v| format!("{:.2}", v))),
            hold: FloatParam::new("Hold", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| format!("{:.2}", v))),
            decay: FloatParam::new("Decay", 0.1, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| format!("{:.2}", v))),
            sustain: FloatParam::new("Sustain", 0.8, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| format!("{:.2}", v))),
            release: FloatParam::new("Release", 0.1, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(Arc::new(|v| format!("{:.2}", v))),
        }
    }
}

impl Plugin for Orbital {
    const NAME: &'static str = "Orbital";
    const VENDOR: &'static str = "Tendsin's Lab";
    const URL: &'static str = "https://siebencorgie.rs";
    const EMAIL: &'static str = "info@example.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // We'll only do stereo for now
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_output_channels: NonZeroU32::new(Orbital::NUM_CHANNELS),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
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
            move |egui_ctx, setter, renderer| renderer.draw(egui_ctx, setter),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        nih_log!("Init");

        //signal polyphony.
        context.set_current_voice_capacity(10);

        //if profiling, add server
        #[cfg(feature = "profile")]
        {
            nih_log!("Setting up profiling!");
            puffin::set_scopes_on(true);
            let server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);
            nih_log!("On: {}", server_addr);
            self.server = Some(puffin_http::Server::new(&server_addr).unwrap());
        }

        //init synth to current state, or default
        self.synth.bank.on_state_change(
            self.params
                .solar_system
                .try_read()
                .map(|lck| lck.get_solar_state())
                .unwrap_or(SolarSystem::new().get_solar_state()),
        );
        self.synth.set_envelopes(self.get_adsr_settings());
        self.synth.bank.mod_ty = self
            .params
            .mod_ty
            .lock()
            .map(|m| m.clone())
            .unwrap_or(ModulationType::default());
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
        #[cfg(feature = "profile")]
        puffin::GlobalProfiler::lock().new_frame();

        #[cfg(feature = "profile")]
        puffin::profile_function!();

        let buffer_length = buffer.samples() as Time / context.transport().sample_rate as f64;
        let sample_time = 1.0 / context.transport().sample_rate as Time;

        //try at most 10
        // TODO: check if we maybe should do that async
        for _try in 0..10 {
            match self.com_channel.1.try_recv() {
                Ok(msg) => match msg {
                    ComMsg::StateChange(s) => self.synth.bank.on_state_change(s),
                    ComMsg::ModRelationChanged(new) => {
                        if let Ok(mut mr) = self.params.mod_ty.try_lock() {
                            *mr = new.clone();
                        }
                        self.synth.bank.mod_ty = new
                    }
                    ComMsg::GainChange(new_gain) => {
                        if let Ok(mut p) = self.params.gain_ty.try_lock() {
                            *p = new_gain.clone();
                        }
                        self.synth.bank.gain_ty = new_gain;
                    }
                },
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

        //Overwrite ADSR
        //TODO: Find out if anything changed. We have two sources for that:
        //      1. From ui (we can track that)
        //      2. From DAW (no idea how to track that)
        self.synth.set_envelopes(self.get_adsr_settings());
        self.synth.bank.reset_phase = self.params.reset_phase.value();

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
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(Orbital);
nih_export_vst3!(Orbital);
