use nih_plug::prelude::Enum;
use serde::{Deserialize, Serialize};

use crate::{
    envelope::EnvelopeParams,
    osc::{sigmoid, ModulationType, OscType},
};

#[derive(Clone, Debug)]
pub struct OrbitalState {
    ///Phase offset of this orbital
    pub offset: f32,
    pub ty: OscType,
    //oscillator slot
    pub slot: usize,
}

#[derive(Clone, Debug)]
pub struct SolarState {
    pub states: Vec<OrbitalState>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Enum)]
pub enum GainType {
    Sigmoid,
    Linear,
}

impl Default for GainType{
    fn default() -> Self {
        GainType::Sigmoid
    }
}

impl GainType {
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
#[derive(Clone, Debug)]
pub enum ComMsg {
    ///new solar state update
    SolarState(SolarState),
    EnvChanged(EnvelopeParams),
    ModRelationChanged(ModulationType),
    GainChange(GainType),
    ResetPhaseChanged(bool)
}
