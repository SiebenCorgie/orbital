use serde::{Deserialize, Serialize};

///Single primary oscillator. Does nothing on its own, but collecting the state.
/// All the logic is implemented in the parent osc.rs or one of the helpers.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct PrimaryOsc {
    ///Base frequency multiplier. This basically means if a note @ 440Hz is played, and this is 0.5, then
    /// the primary oscillator has a base frequency of 220Hz
    pub speed_index: f32,
    pub volume: f32,
    pub is_on: bool,
}

impl PrimaryOsc {
    #[inline(always)]
    pub fn freq(&self, base_frequency: f32) -> f32 {
        base_frequency * 2.0f32.powf(self.speed_index)
    }
}

impl Default for PrimaryOsc {
    fn default() -> Self {
        PrimaryOsc {
            speed_index: 0.0,
            volume: 0.0,
            is_on: false,
        }
    }
}
