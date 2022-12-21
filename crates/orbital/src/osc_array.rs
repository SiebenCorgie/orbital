use crate::{osc::OscillatorBank, envelope::Envelope};


///Single banks state.
pub struct OscVoiceState{
    //local voice's envelope state.
    env: Envelope,
}

impl Default for OscVoiceState {
    fn default() -> Self {
        OscVoiceState { env: Envelope::default() }
    }
}


///Oscillator bank array. Basically, if you imagine a grid of oscillators, each bank is a
/// column (per voice).
///
/// This is more or less *the synth*.
pub struct OscArray{
    //all os
    banks: OscillatorBank,
    voices: [OscVoiceState; OscillatorBank::VOICE_COUNT],
}
