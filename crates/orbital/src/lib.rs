use atomic_float::AtomicF32;
use com::ComMsg;
use crossbeam::channel::{Sender, Receiver, TryRecvError};
use nih_plug::{prelude::*, params::persist, util::midi_note_to_freq};
use nih_plug_egui::{create_egui_editor, egui::{self, Painter}, EguiState};
use osc::OscillatorBank;
use renderer::{Renderer, solar_system::SolarSystem};
use std::sync::{Arc, Mutex};


mod renderer;
mod osc;
mod com;
/// The time it takes for the peak meter to decay by 12 dB after switching to complete silence.
const PEAK_METER_DECAY_MS: f64 = 150.0;

/// This is mostly identical to the gain example, minus some fluff, and with a GUI.
pub struct Orbital {
    params: Arc<OrbitalParams>,

    com_channel: (Sender<ComMsg>, Receiver<ComMsg>),
    /// Needed to normalize the peak meter's response based on the sample rate.
    peak_meter_decay_weight: f32,
    /// The current data for the peak meter. This is stored as an [`Arc`] so we can share it between
    /// the GUI and the audio processing parts. If you have more state to share, then it's a good
    /// idea to put all of that in a struct behind a single `Arc`.
    ///
    /// This is stored as voltage gain.
    peak_meter: Arc<AtomicF32>,

    ///in audio-thread osc bank
    bank: OscillatorBank
}



#[derive(Params)]
pub struct OrbitalParams {
    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    #[id = "gain"]
    pub gain: FloatParam,

    #[persist = "OscBank"]
    pub bank: Arc<Mutex<OscillatorBank>>,
    #[persist = "SolarSystem"]
    pub solar_system: Arc<Mutex<SolarSystem>>,
}

impl Default for Orbital {
    fn default() -> Self {
        Self {
            params: Arc::new(OrbitalParams::default()),
            com_channel: crossbeam::channel::unbounded(),
            peak_meter_decay_weight: 1.0,
            peak_meter: Arc::new(AtomicF32::new(util::MINUS_INFINITY_DB)),
            bank: OscillatorBank::default()
        }
    }
}

impl Default for OrbitalParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(640, 480),

            // See the main gain example for more details
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            bank: Arc::new(Mutex::new(OscillatorBank::default())),
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
        let peak_meter = self.peak_meter.clone();
        let renderer = Renderer::new(params, self.com_channel.0.clone());
        create_egui_editor(
            self.params.editor_state.clone(),
            renderer,
            |_, _| {},
            move |egui_ctx, setter, renderer| {
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
        // After `PEAK_METER_DECAY_MS` milliseconds of pure silence, the peak meter's value should
        // have dropped by 12 dB
        self.peak_meter_decay_weight = 0.25f64
            .powf((buffer_config.sample_rate as f64 * PEAK_METER_DECAY_MS / 1000.0).recip())
            as f32;

        nih_log!("Init");
        true
    }


    fn deactivate(&mut self) {
        //feed back current parameter state
        if let Ok(mut lck) = self.params.bank.lock(){
            *lck = self.bank.clone();
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


        //try at most 10
        // TODO: check if we maybe should do that async
        for _try in 0..10{
            match self.com_channel.1.try_recv(){
                Ok(msg) => {
                    nih_log!("Got: {:?}", msg);
                    self.bank.on_msg(msg);
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

        while let Some(ev) = context.next_event(){
            match ev{
                NoteEvent::NoteOn { note, .. } => self.bank.speed_multiplier = midi_note_to_freq(note),
                NoteEvent::NoteOff { .. } => self.bank.speed_multiplier = 0.0,
                _ => {}
            }
        }

        self.bank.process(buffer, context.transport().sample_rate);

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Orbital {
    const CLAP_ID: &'static str = "com.tendsins-lab.orbital";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Cosmic FM-Synth");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for Orbital {
    const VST3_CLASS_ID: [u8; 16] = *b"OrbitalSynthnnns";
    const VST3_CATEGORIES: &'static str = "Instrument|Synth";
}

nih_export_clap!(Orbital);
nih_export_vst3!(Orbital);
