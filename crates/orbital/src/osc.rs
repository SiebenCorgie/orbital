use nih_plug::{nih_error, prelude::{Buffer, Enum}};
use serde::{Deserialize, Serialize};

use crate::{com::{ComMsg, OrbitalState}, renderer::orbital::{TWOPI, Orbital}};

pub fn sigmoid(x: f32) -> f32{
    x / (1.0 + x * x).sqrt()
}

pub fn mel_to_freq(mel: f32) -> f32 {
    700.0 * (10.0f32.powf((mel + 20.0) / 2595.0) - 1.0)
}

pub fn freq_to_mel(freq: f32) -> f32 {
    2595.0 * (1.0 + (freq / 700.0)).log10()
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Enum)]
pub enum ModulationType{
    Absolute,
    Relative
}

///There are two oscillator types, primary, and modulator. However we also track turned off osc's
/// for performance reasons
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OscType{
    Primary{
        ///Base frequency multiplier. This basically means if a note @ 440Hz is played, and this is 0.5, then
        /// the primary oscillator has a base frequency of 220Hz
        base_multiplier: f32,
        volume: f32,
    },
    ///Modulates the Oscillator at the given index in the bank.
    Modulator{
        ///the parent oscillator
        parent_osc_slot: usize,
        ///The modulation range in % of the parents frequency. At 0 no modulation happens, at 1.0 the value is modulated +/- 100%
        ///
        /// The modulation speed is determined by the own self.speed, the current amount (weighted by the percentile) is
        /// calculated by advance function.
        range: f32,
        ///Abstract speed of this modulator. Depending on the modulation type this is
        /// either the relative frequency modulation, or a certain frequency in mel.
        speed: f32
    },
    Off
}

impl OscType{

    ///Calculates a step value based on:
    ///
    /// δ_sec: time in seconds each sample takes (1.0 / sample_rate)
    /// base_frequency: the frequency of the key that is played.
    /// frequency_multiplier: multiplier calculated from children's modulation.
    ///
    /// The returned value is in radiant, aka "parts on the oscillators circle"
    #[inline]
    fn phase_step(&self, d_sec: f32, base_frequency: f32, frequency_multiplier: f32, mod_ty: &ModulationType) -> f32{
        match self{
            OscType::Primary { base_multiplier, .. } => {
                //calculate step by finding "our" base frequency, and weighting that with the percentile. Then advance
                // via δ
                let local_base = (base_frequency * base_multiplier).max(0.0);
                let final_freq = local_base * frequency_multiplier;
                d_sec * final_freq * TWOPI
            },
            OscType::Modulator { parent_osc_slot: _, range: _, speed } => {
                //depending on the modulation type, either scale by base frequency, or dont't
                match mod_ty {
                //in this case, its easy, weigh with percentile and move base on our frequency
                    ModulationType::Absolute => d_sec * mel_to_freq(speed * Orbital::MEL_MULTIPLIER) * frequency_multiplier * TWOPI,
                    ModulationType::Relative => {

                        //what we want is to take the base frequency of the tone, and modulate it with the current speed.
                        // `speed` is is -inf..inf. We translate the absolute on mel to mel, depending on the sign either
                        // add or subtract from the base frequency (in mel) and translate that back into hz.
                        //
                        // Then we add the modulation multiplier as well.



                        let modded_base = mel_to_freq((freq_to_mel(base_frequency) * *speed).max(0.0));

                        d_sec * modded_base * frequency_multiplier * TWOPI
                    }
                }

            },
            OscType::Off => {0.0}
        }
    }

    fn is_primary(&self) -> bool{
        if let OscType::Primary {..} = self{
            true
        }else{
            false
        }
    }

    fn is_off(&self) -> bool{
        if let OscType::Off = self{
            true
        }else{
            false
        }
    }

    fn volume(&self) -> f32{
        if let OscType::Primary {volume, .. } = self{
            *volume
        }else{
            1.0
        }
    }
}

/// Single oscillator state. Used to sync graphics and audio engine as well as
/// saving the state
#[derive(Serialize, Deserialize, Clone)]
pub struct Oscillator{
    ///Combined modulation percentile. When generation the next value the base_frequency
    /// is slowed down/ speed up by this
    mod_multiplier: f32,
    ///While updating, counts number of children, to make sense of the multiplier.
    /// If this is 0 we also know that we can ignore the multiplier
    mod_counter: usize,
    ///Phase offset (0..2π)
    offset: f32,
    ///last known phase of the osc (0..2π) in radiant.
    phase: f32,
    ty: OscType
}

impl Oscillator{
    fn freq_multiplier(&self) -> f32{
        if self.mod_counter == 0{
            1.0
        }else{
            //NOTE: has to be at least 1 in divisor
            self.mod_multiplier / self.mod_counter as f32
        }
    }

    fn sample(&self) -> f32{
        (self.phase + self.offset).cos() * self.ty.volume()
    }
}

impl Default for Oscillator{
    fn default() -> Self {
        Oscillator {
            mod_multiplier: 1.0,
            mod_counter: 0,
            offset: 0.0,
            phase: 0.0,
            ty: OscType::Off,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct OscillatorBank{
    ///Stores *all* oscillators. The children are declared in form of indices within the osc structs.
    oscillators: Vec<Oscillator>,
    pub speed_multiplier: f32,
    pub mod_ty: ModulationType,
    phase: f32,
}

impl Default for OscillatorBank{
    fn default() -> Self {
        //pre allocating oscillator banks. But vec allows us to outgrow if neede
        OscillatorBank {
            oscillators: vec![Oscillator::default(); Self::OSC_COUNT],
            speed_multiplier: 0.0,
            mod_ty: ModulationType::Absolute,
            phase: 0.0,
        }
    }
}

impl OscillatorBank{
    ///Number of maximal active voices.
    pub const VOICE_COUNT: usize = 10;
    ///Number of oscillators per voice.
    pub const OSC_COUNT: usize = 16;

    pub fn on_msg(&mut self, msg: ComMsg){
        match msg {
            ComMsg::SolarState(new_state) => {

                //turn off all to not keep anything "on" by misstake.
                for o in &mut self.oscillators{
                    o.ty = OscType::Off;
                }

                //reconifg all oscs
                // TODO: do diff and lerp between changes, reset on type change
                for (idx, state) in new_state.states.into_iter().enumerate(){
                    let OrbitalState { offset, ty } = state;
                    if let Some(osc) = self.oscillators.get_mut(idx){
                        osc.ty = ty;
                        osc.offset = offset;
                    }else{
                        nih_error!("outside oscillator bank, allocating!");
                    }

                }
            },
            ComMsg::SpeedMultiplier(new) => self.speed_multiplier = new,
        }
    }

    ///Steps the whole bank once, returning a modulated value based on "base_frequency".
    fn step(&mut self, base_frequency: f32, sample_delta: f32) -> f32{
        //we have two stepping procedures. One is the "high resolution"
        // phase.cos() for base osciis, and the lower resolution LFO type cos-less approximation.
        // TODO: implement https://www.cl.cam.ac.uk/~am21/hakmemc.html @ 151

        //Accumulates all primary values
        let mut accumulated = 0.0;
        let mut div = 0;
        //atm. step all
        for osc_idx in 0..self.oscillators.len(){
            if self.oscillators[osc_idx].ty.is_off(){
                continue;
            }
            //advance osc state
            {
                let osc = &mut self.oscillators[osc_idx];
                let osc_adv = osc.ty.phase_step(sample_delta, base_frequency, osc.freq_multiplier(), &self.mod_ty);
                osc.phase = (osc.phase + osc_adv) % TWOPI;
                //reset modulator for next iteration, since we just used the old value
                osc.mod_counter = 0;
                osc.mod_multiplier = 0.0;
                //if we are a primary osc, add to acc
                match osc.ty{
                    OscType::Primary { .. } => {
                        accumulated += self.oscillators[osc_idx].sample();
                        div += 1;
                    },
                    _ => {}
                }
            };
        }

        //now update all parents of any secondary osc.
        // NOTE: Can't do that in the first loop, since any osc might only be partially updated at that point.
        // TODO: maybe sort bank in a way that we can do that at once?
        for i in 0..self.oscillators.len(){
            if self.oscillators[i].ty.is_off(){
                continue;
            }
            let update = if let OscType::Modulator { parent_osc_slot, range, .. } = &self.oscillators[i].ty {
                //NOTE: we got a phase for the mod oscillator. However the cos is (-1 .. 1). So we weight by range into (-range .. range).
                //      Next we want to only modulate the range around (100% - range .. 100% + range), so we add 1
                let modulatio_value = (self.oscillators[i].sample() * range) + 1.0;
                Some((*parent_osc_slot, modulatio_value))
            }else{
                None
            };

            if let Some((parent, multiplier)) = update{
                //if we got anything, update modulator
                self.oscillators[parent].mod_counter += 1;
                self.oscillators[parent].mod_multiplier += multiplier;
            }
        }

        //now update
        sigmoid(accumulated)
    }

    //Fills the buffer with sound jo
    pub fn process(&mut self, buffer: &mut Buffer, sample_rate: f32){
        let delta_sec = 1.0 / sample_rate;
        for mut sample in buffer.iter_samples(){
            let val = self.step(self.speed_multiplier, delta_sec);
            for csam in sample.iter_mut(){
                *csam = val;
            }
        }
    }
}
