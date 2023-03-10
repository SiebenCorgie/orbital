use nih_plug::{prelude::Buffer, util::midi_note_to_freq};
use serde::{Deserialize, Serialize};

use crate::{
    envelope::{Envelope, EnvelopeParams},
    osc::OscillatorBank,
    Time,
};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum VoiceState {
    Off,
    On,
    Released,
}

impl VoiceState {
    pub fn is_off(&self) -> bool {
        if let VoiceState::Off = &self {
            true
        } else {
            false
        }
    }
    pub fn is_released(&self) -> bool {
        if let VoiceState::Released = &self {
            true
        } else {
            false
        }
    }

    pub fn is_active(&self) -> bool {
        if let Self::Off = self {
            false
        } else {
            true
        }
    }
}

///Single banks state.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct OscVoiceState {
    //local voice's envelope state.
    pub env: Envelope,
    pub state: VoiceState,
    pub note: u8,
    pub freq: f32,
}

impl Default for OscVoiceState {
    fn default() -> Self {
        OscVoiceState {
            env: Envelope::default(),
            state: VoiceState::Off,
            note: 0,
            freq: 0.0,
        }
    }
}

///Oscillator bank array. Basically, if you imagine a grid of oscillators, each bank is a
/// column (per voice).
///
/// This is more or less *the synth*.
#[derive(Serialize, Deserialize, Clone)]
pub struct OscArray {
    //all os
    pub bank: OscillatorBank,
    voices: [OscVoiceState; OscillatorBank::VOICE_COUNT],
}

impl Default for OscArray {
    fn default() -> Self {
        OscArray {
            bank: OscillatorBank::default(),
            voices: [OscVoiceState::default(); OscillatorBank::VOICE_COUNT],
        }
    }
}

impl OscArray {
    pub fn note_on(&mut self, note: u8, at: Time) {
        //search for an inactive voice and init.
        for (vidx, v) in self.voices.iter_mut().enumerate() {
            if v.state.is_off() {
                v.state = VoiceState::On;
                v.note = note;
                v.freq = midi_note_to_freq(note);
                v.env.on_press(at);

                if self.bank.reset_phase {
                    self.bank.reset_voice(vidx);
                }

                return;
            }
        }
    }

    pub fn note_off(&mut self, note: u8, at: Time) {
        for v in &mut self.voices {
            if v.note == note && !v.state.is_off() {
                v.env.on_release(at);
                v.state = VoiceState::Released;
            }
        }
    }

    pub fn set_envelopes(&mut self, new: EnvelopeParams) {
        for v in &mut self.voices {
            v.env.parameters = new.clone();
        }
    }

    pub fn process(&mut self, buffer: &mut Buffer, sample_rate: f32, buffer_time_start: Time) {
        #[cfg(feature = "profile")]
        puffin::profile_function!("synth main process");
        //check each voice once if we can turn it off
        for v in &mut self.voices {
            #[cfg(feature = "profile")]
            puffin::profile_scope!("Voice key-filter update");
            if v.env.after_sampling(buffer_time_start) {
                v.state = VoiceState::Off;
                v.env.reset();
                v.freq = 0.0;
                v.note = 0;
            }
        }
        //fire process
        self.bank
            .process(&self.voices, buffer, sample_rate, buffer_time_start);
    }
}
