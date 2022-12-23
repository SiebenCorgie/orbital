use crate::{
    envelope::EnvelopeParams,
    osc::{ModulationType, OscType},
};

#[derive(Clone, Debug)]
pub struct OrbitalState {
    pub offset: f32,
    pub ty: OscType,
    //oscillator slot
    pub slot: usize,
}

#[derive(Clone, Debug)]
pub struct SolarState {
    pub states: Vec<OrbitalState>,
}

///Communication messages from the renderer to the oscillator bank.
#[derive(Clone, Debug)]
pub enum ComMsg {
    ///new solar state update
    SolarState(SolarState),
    EnvChanged(EnvelopeParams),
    ModRelationChanged(ModulationType),
}
