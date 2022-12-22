use nih_plug::{prelude::Buffer, util::midi_note_to_freq};
use serde::{Deserialize, Serialize};

use crate::{osc::OscillatorBank, envelope::Envelope, Time};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum VoiceState{
    Off,
    On,
    Released,
}

impl VoiceState{
    pub fn is_off(&self) -> bool{
        if let VoiceState::Off = &self{
            true
        }else{
            false
        }
    }
    pub fn is_released(&self) -> bool{
        if let VoiceState::Released = &self{
            true
        }else{
            false
        }
    }

    pub fn is_active(&self) -> bool{
        if let Self::Off = self{
            false
        }else{
            true
        }
    }
}

///Single banks state.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct OscVoiceState{
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
pub struct OscArray{
    //all os
    pub bank: OscillatorBank,
    voices: [OscVoiceState; OscillatorBank::VOICE_COUNT],
}

impl Default for OscArray{
    fn default() -> Self {
        OscArray {
            bank: OscillatorBank::default(),
            voices: [OscVoiceState::default(); OscillatorBank::VOICE_COUNT]
        }
    }
}


impl OscArray{
    pub fn note_on(&mut self, note: u8, at: Time){
        //search for an inactive voice and init.
        for v in &mut self.voices{
            if !v.state.is_active(){
                v.state = VoiceState::On;
                v.note = note;
                v.freq = midi_note_to_freq(note);
                v.env.on_press(at);
                return;
            }
        }
    }

    pub fn note_off(&mut self, note: u8, at: Time){
        for v in &mut self.voices{
            if v.note == note{
                v.env.on_release(at);
                v.state = VoiceState::Released;
            }
        }
    }

    pub fn process(&mut self, buffer: &mut Buffer, sample_rate: f32, buffer_time_start: Time, buffer_time_length: Time){
        //check each voice once if we can turn it off
        for v in &mut self.voices{
            if v.state.is_released() && v.env.sample(buffer_time_start) <= 0.0{
                v.state = VoiceState::Off;
            }
        }
        //fire process
        self.bank.process(&self.voices, buffer, sample_rate, buffer_time_start, buffer_time_length);
    }
}
