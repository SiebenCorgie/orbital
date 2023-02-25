use nih_plug::prelude::Enum;
use serde::{Deserialize, Serialize};

use crate::{
    envelope::EnvelopeParams,
    osc::{modulator::ModulatorOsc, primary::PrimaryOsc, sigmoid, ModulationType},
};

#[derive(Clone)]
pub struct SolarState {
    pub primary_states: Vec<PrimaryState>,
    pub modulator_states: Vec<ModulatorState>,
}

#[derive(Clone)]
pub struct PrimaryState {
    pub offset: f32,
    pub state: PrimaryOsc,
    pub slot: usize,
}

#[derive(Clone)]
pub struct ModulatorState {
    pub offset: f32,
    pub state: ModulatorOsc,
    pub slot: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Enum)]
pub enum GainType {
    Sigmoid,
    Linear,
}

impl Default for GainType {
    fn default() -> Self {
        GainType::Sigmoid
    }
}

impl GainType {
    #[inline(always)]
    pub fn map(&self, value: f32) -> f32 {
        match self {
            GainType::Sigmoid => sigmoid(value),
            GainType::Linear => value.clamp(-1.0, 1.0),
        }
    }

    pub fn next(&mut self) {
        match self {
            GainType::Linear => *self = GainType::Sigmoid,
            GainType::Sigmoid => *self = GainType::Linear,
        }
    }
}

///Communication messages from the renderer to the oscillator bank.
#[derive(Clone)]
pub enum ComMsg {
    StateChange(SolarState),
    ModRelationChanged(ModulationType),
    GainChange(GainType),
    ResetPhaseChanged(bool),
}
