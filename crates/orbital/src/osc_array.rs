use nih_plug::{prelude::Buffer, util::midi_note_to_freq};
use serde::{Deserialize, Serialize};

use crate::{osc::OscillatorBank, envelope::Envelope};


///Single banks state.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct OscVoiceState{
    //local voice's envelope state.
    env: Envelope,
    is_active: bool,
    note: u8,
}

impl Default for OscVoiceState {
    fn default() -> Self {
        OscVoiceState {
            env: Envelope::default(),
            is_active: false,
            note: 0
        }
    }
}

///All info needed to sample a voice.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct VoiceSampling{
    pub is_active: bool,
    pub volume: f32,
    pub base_frequency: f32,
}

impl Default for VoiceSampling{
    fn default() -> Self {
        VoiceSampling { is_active: false, volume: 1.0, base_frequency: 0.0 }
    }
}
///Oscillator bank array. Basically, if you imagine a grid of oscillators, each bank is a
/// column (per voice).
///
/// This is more or less *the synth*.
#[derive(Serialize, Deserialize, Clone)]
pub struct OscArray{
    //all os
    pub bank: OscillatorBank,
    voice_sampling: [VoiceSampling; OscillatorBank::VOICE_COUNT],
    voices: [OscVoiceState; OscillatorBank::VOICE_COUNT],
}

impl Default for OscArray{
    fn default() -> Self {
        OscArray {
            bank: OscillatorBank::default(),
            voice_sampling: [VoiceSampling::default(); OscillatorBank::VOICE_COUNT],
            voices: [OscVoiceState::default(); OscillatorBank::VOICE_COUNT]
        }
    }
}


impl OscArray{
    pub fn note_on(&mut self, note: u8, at: f32){
        //search for an inactive voice and init.
        for v in &mut self.voices{
            if !v.is_active{
                v.is_active = true;
                v.note = note;
                v.env.on_press(at);
                return;
            }
        }
    }

    pub fn note_off(&mut self, note: u8, at: f32){
        for v in &mut self.voices{
            if v.note == note{
                v.is_active = false;
            }
        }
    }

    pub fn process(&mut self, buffer: &mut Buffer, sample_rate: f32){
        //prepare voice info
        for v in 0..OscillatorBank::VOICE_COUNT{
            if self.voices[v].is_active{
                self.voice_sampling[v].base_frequency = midi_note_to_freq(self.voices[v].note);
                self.voice_sampling[v].is_active = true;
                self.voice_sampling[v].volume = 1.0;

            }else{
                self.voice_sampling[v].is_active = false;
            }
        }

        //fire process
        self.bank.process(&self.voice_sampling, buffer, sample_rate);
    }
}
