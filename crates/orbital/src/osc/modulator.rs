use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParentIndex {
    Primary(usize),
    Modulator(usize),
}

///Single primary oscillator. Does nothing on its own, but collecting the state.
/// All the logic is implemented in the parent osc.rs or one of the helpers.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct ModulatorOsc {
    pub parent_osc_slot: ParentIndex,
    pub is_on: bool,
    ///The modulation range in % of the parents frequency. At 0 no modulation happens, at 1.0 the value is modulated +/- 100%
    ///
    /// The modulation speed is determined by the own self.speed, the current amount (weighted by the percentile) is
    /// calculated by advance function.
    pub range: f32,
    ///Abstract speed of this modulator. Depending on the modulation type this is
    /// either the relative frequency modulation, or a certain frequency in mel.
    pub speed_index: f32,
}

impl ModulatorOsc {
    #[inline(always)]
    pub fn freq(&self, base_frequency: f32) -> f32 {
        base_frequency * 2.0f32.powf(self.speed_index)
    }
}

impl Default for ModulatorOsc {
    fn default() -> Self {
        ModulatorOsc {
            parent_osc_slot: ParentIndex::Primary(0),
            is_on: false,
            range: 0.0,
            speed_index: 0.0,
        }
    }
}
