use std::simd;

use nih_plug::prelude::{Buffer, Enum};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    com::{GainType, ModulatorState, PrimaryState, SolarState},
    osc::modulator::ParentIndex,
    osc_array::OscVoiceState,
    renderer::orbital::{Orbital, TWOPI},
    Time,
};

use self::{modulator::ModulatorOsc, primary::PrimaryOsc};

pub mod modulator;
pub mod primary;

#[inline(always)]
pub fn sigmoid(x: f32) -> f32 {
    x / (1.0 + x * x).sqrt()
}

#[allow(dead_code)]
pub fn mel_to_freq(mel: f32) -> f32 {
    700.0 * (10.0f32.powf((mel + 20.0) / 2595.0) - 1.0)
}

#[allow(dead_code)]
pub fn freq_to_mel(freq: f32) -> f32 {
    2595.0 * (1.0 + (freq / 700.0)).log10()
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Enum)]
pub enum ModulationType {
    Absolute,
    Relative,
}

impl Default for ModulationType {
    fn default() -> Self {
        ModulationType::Relative
    }
}

impl ModulationType {
    pub fn next(&self) -> Self {
        match self {
            ModulationType::Absolute => Self::Relative,
            ModulationType::Relative => Self::Absolute,
        }
    }
}

///There are two oscillator types, primary, and modulator. However we also track turned off osc's
/// for performance reasons
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum OscType {
    Primary {
        ///Base frequency multiplier. This basically means if a note @ 440Hz is played, and this is 0.5, then
        /// the primary oscillator has a base frequency of 220Hz
        speed_index: i32,
        volume: f32,
    },
    ///Modulates the Oscillator at the given index in the bank.
    Modulator {
        ///the parent oscillator
        parent_osc_slot: usize,
        ///The modulation range in % of the parents frequency. At 0 no modulation happens, at 1.0 the value is modulated +/- 100%
        ///
        /// The modulation speed is determined by the own self.speed, the current amount (weighted by the percentile) is
        /// calculated by advance function.
        range: f32,
        ///Abstract speed of this modulator. Depending on the modulation type this is
        /// either the relative frequency modulation, or a certain frequency in mel.
        speed_index: i32,
    },
    Off,
}

/*NOTE: this is dead code from the non-simd implementation of the oscillators. Keeping it here just in case
     * we want to reimplement an non-simd path later.
impl OscType {
    //returns current frequency in relation to a base frequency
    fn freq(&self, base_frequency: f32) -> f32 {
        match self {
            Self::Modulator { speed_index, .. } => {
                base_frequency * 2.0f32.powf(*speed_index as f32)
            }
            Self::Primary { speed_index, .. } => base_frequency * 2.0f32.powf(*speed_index as f32),
            Self::Off => 0.0,
        }
    }

    ///Calculates a step value based on:
    ///
    /// δ_sec: time in seconds each sample takes (1.0 / sample_rate)
    /// base_frequency: the frequency of the key that is played.
    /// frequency_multiplier: multiplier calculated from children's modulation.
    ///
    /// The returned value is in radiant, aka "parts on the oscillators circle"
    #[inline]
    fn phase_step(
        &self,
        d_sec: f32,
        base_frequency: f32,
        frequency_multiplier: f32,
        mod_ty: &ModulationType,
    ) -> f32 {
        match self {
            OscType::Primary { .. } => {
                //calculate step by finding "our" base frequency, and weighting that with the percentile. Then advance
                // via δ
                let local_base = self.freq(base_frequency).max(0.0);
                let final_freq = local_base * frequency_multiplier;
                d_sec * final_freq * TWOPI
            }
            OscType::Modulator {
                parent_osc_slot: _,
                range: _,
                ..
            } => {
                //depending on the modulation type, either scale by base frequency, or don't
                match mod_ty {
                    //in this case, its easy, weigh with percentile and move base on our frequency
                    ModulationType::Absolute => {
                        d_sec * self.freq(Orbital::ABS_BASE_FREQ) * frequency_multiplier * TWOPI
                    }
                    ModulationType::Relative => {
                        //what we want is to take the base frequency of the tone, and modulate it with the current speed.
                        // `speed` is is -inf..inf. We translate the absolute on mel to mel, depending on the sign either
                        // add or subtract from the base frequency (in mel) and translate that back into hz.
                        //
                        // Then we add the modulation multiplier as well.

                        let modded_base = self.freq(base_frequency).max(0.0);

                        d_sec * modded_base * frequency_multiplier * TWOPI
                    }
                }
            }
            OscType::Off => 0.0,
        }
    }

    fn is_off(&self) -> bool {
        if let OscType::Off = self {
            true
        } else {
            false
        }
    }

    fn volume(&self) -> f32 {
        if let OscType::Primary { volume, .. } = self {
            *volume
        } else {
            1.0
        }
    }
}
*/

/// Single oscillator state. Used to sync graphics and audio engine as well as
/// saving the state
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Oscillator<S> {
    //Oscillator state type
    osc: S,
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
}

impl<S> Oscillator<S> {
    fn freq_multiplier(&self) -> f32 {
        if self.mod_counter == 0 {
            1.0
        } else {
            //NOTE: has to be at least 1 in divisor
            self.mod_multiplier / self.mod_counter as f32
        }
    }

    /*
    #[inline(always)]
    fn sample(&self) -> f32 {
        #[cfg(feature = "profile")]
        puffin::profile_function!();
        (self.phase + self.offset).cos() * self.ty.volume()
    }
    */
}

impl<S: Default> Default for Oscillator<S> {
    fn default() -> Self {
        Oscillator {
            osc: S::default(),
            mod_multiplier: 1.0,
            mod_counter: 0,
            offset: 0.0,
            phase: 0.0,
        }
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone)]
pub struct OscillatorBank {
    ///Stores *all* primary oscillators. The children are declared in form of indices within the osc structs.
    #[serde_as(as = "[_; OscillatorBank::PRIMARY_BANK_SIZE]")]
    primary_osc: [Oscillator<PrimaryOsc>; Self::PRIMARY_BANK_SIZE],
    #[serde_as(as = "[_; OscillatorBank::MODULATOR_BANK_SIZE]")]
    modulator_osc: [Oscillator<ModulatorOsc>; Self::MODULATOR_BANK_SIZE],
    pub mod_ty: ModulationType,
    pub gain_ty: GainType,
    pub reset_phase: bool,
}

impl Default for OscillatorBank {
    fn default() -> Self {
        //pre allocating oscillator banks. But vec allows us to outgrow if neede
        OscillatorBank {
            primary_osc: [Oscillator::default(); Self::PRIMARY_BANK_SIZE],
            modulator_osc: [Oscillator::default(); Self::MODULATOR_BANK_SIZE],
            mod_ty: ModulationType::default(),
            gain_ty: GainType::default(),
            reset_phase: false,
        }
    }
}

impl OscillatorBank {
    ///Number of maximal active voices.
    pub const VOICE_COUNT: usize = 10;
    ///Number of primary oscillators per voice.
    pub const PRIMARY_OSC_COUNT: usize = 8;
    ///Number of modulator oscillators per voice.
    pub const MOD_OSC_COUNT: usize = 16;

    pub const PRIMARY_BANK_SIZE: usize = Self::VOICE_COUNT * Self::PRIMARY_OSC_COUNT;
    pub const MODULATOR_BANK_SIZE: usize = Self::VOICE_COUNT * Self::MOD_OSC_COUNT;

    pub fn on_state_change(&mut self, new: SolarState) {
        //nih_log!("State change");

        //turn off all to not keep anything "on" by misstake.
        for o in &mut self.primary_osc {
            o.osc.is_on = false;
        }

        for o in &mut self.modulator_osc {
            o.osc.is_on = false;
        }

        //reconifg all oscs
        // TODO: do diff and lerp between changes, reset on type change

        for pstate in new.primary_states {
            let PrimaryState {
                offset,
                state,
                slot,
            } = pstate;
            //nih_log!("  [{}]: {:?}", slot, state);
            self.on_primary_osc_line(slot, |osc| {
                osc.offset = offset;
                osc.osc = state;
            })
        }

        for pstate in new.modulator_states {
            let ModulatorState {
                offset,
                state,
                slot,
            } = pstate;

            //nih_log!("  [{}]: {:?}", slot, state);
            self.on_modulator_osc_line(slot, |osc| {
                osc.offset = offset;
                osc.osc = state;
            })
        }
    }

    fn on_primary_osc_line(&mut self, line: usize, f: impl Fn(&mut Oscillator<PrimaryOsc>)) {
        if line >= Self::PRIMARY_OSC_COUNT {
            return;
        }

        for vidx in 0..Self::VOICE_COUNT {
            f(&mut self.primary_osc[Self::primary_osc_index(vidx, line)]);
        }
    }

    fn on_modulator_osc_line(&mut self, line: usize, f: impl Fn(&mut Oscillator<ModulatorOsc>)) {
        if line >= Self::MOD_OSC_COUNT {
            return;
        }

        for vidx in 0..Self::VOICE_COUNT {
            f(&mut self.modulator_osc[Self::modulator_osc_index(vidx, line)]);
        }
    }

    #[inline(always)]
    fn primary_osc_index(voice: usize, osc: usize) -> usize {
        voice * Self::PRIMARY_OSC_COUNT + osc
    }

    #[inline(always)]
    fn modulator_osc_index(voice: usize, osc: usize) -> usize {
        voice * Self::MOD_OSC_COUNT + osc
    }
    pub fn reset_voice(&mut self, voice_idx: usize) {
        //nih_log!("Resetting {}", voice_idx);
        for i in 0..Self::PRIMARY_OSC_COUNT {
            let osc = &mut self.primary_osc[Self::primary_osc_index(voice_idx, i)];
            osc.phase = 0.0;
        }

        for i in 0..Self::MOD_OSC_COUNT {
            let osc = &mut self.modulator_osc[Self::modulator_osc_index(voice_idx, i)];
            osc.phase = 0.0;
        }
    }

    //do primary step, returns new phases
    fn phase_step(
        bases: simd::f32x4,          //base frequency this step is derived from
        multiplier: simd::f32x4,     //osc specific speed-up
        current_phases: simd::f32x4, //current osc's phase
        d_sec: f32,                  //time per sample
    ) -> simd::f32x4 {
        let two_pi = simd::f32x4::splat(TWOPI);
        let final_freq = bases * multiplier;

        let delta_phases = simd::f32x4::splat(d_sec) * final_freq * two_pi;

        sleef::f32x::fmodf(current_phases + delta_phases, two_pi)
    }

    #[inline(always)]
    fn simd_sample(phases: simd::f32x4, offsets: simd::f32x4, volume: simd::f32x4) -> simd::f32x4 {
        sleef::f32x::cos_u10(phases + offsets) * volume
    }

    #[inline(always)]
    fn primary_sample(phases: simd::f32x4, offsets: simd::f32x4, volume: simd::f32x4) -> f32 {
        let res = Self::simd_sample(phases, offsets, volume);
        res[0] + res[1] + res[2] + res[3]
    }

    /*
    ///Steps the whole voice-bank once, returning a modulated value based on "base_frequency".
    fn step_scalar(&mut self, voice: usize, base_frequency: f32, sample_delta: f32) -> f32 {
        //Accumulates all primary values
        let mut accumulated = 0.0;
        let mut div = 0;

        //atm. step all
        for osc_idx in 0..Self::OSC_COUNT {
            #[cfg(feature = "profile")]
            puffin::profile_scope!("Phase step");

            let osc_idx = Self::osc_index(voice, osc_idx);
            if self.oscillators[osc_idx].ty.is_off() {
                continue;
            }
            //advance osc state
            {
                let osc = &mut self.oscillators[osc_idx];
                let osc_adv = osc.ty.phase_step(
                    sample_delta,
                    base_frequency,
                    osc.freq_multiplier(),
                    &self.mod_ty,
                );
                osc.phase = (osc.phase + osc_adv) % TWOPI;
                //reset modulator for next iteration, since we just used the old value
                osc.mod_counter = 0;
                osc.mod_multiplier = 0.0;
                //if we are a primary osc, add to acc
                match osc.ty {
                    OscType::Primary { .. } => {
                        accumulated += osc.sample();
                        div += 1;
                    }
                    _ => {}
                }
            };
        }

        //now update all parents of any secondary osc.
        // NOTE: Can't do that in the first loop, since any osc might only be partially updated at that point.

        //For simd purpose we collect all "to merge" oscillators
        // and whenever we fill a line we execute a simd'd sample step and write that back

        for osc_idx in 0..Self::OSC_COUNT {
            #[cfg(feature = "profile")]
            puffin::profile_scope!("Modulator merge");

            let osc_idx = Self::osc_index(voice, osc_idx);
            if self.oscillators[osc_idx].ty.is_off() {
                continue;
            }
            if let OscType::Modulator {
                parent_osc_slot,
                range,
                ..
            } = self.oscillators[osc_idx].ty
            {
                #[cfg(feature = "profile")]
                puffin::profile_scope!("PerModMerge");
                //NOTE: we got a phase for the mod oscillator. However the cos is (-1 .. 1). So we weight by range into (-range .. range).
                //      Next we want to only modulate the range around (100% - range .. 100% + range), so we add 1
                let modulatio_value = (self.oscillators[osc_idx].sample() * range) + 1.0;
                let parent_osc = Self::osc_index(voice, parent_osc_slot);
                self.oscillators[parent_osc].mod_counter += 1;
                self.oscillators[parent_osc].mod_multiplier += modulatio_value;
            }
        }

        //we normalise "per voice"
        accumulated / div as f32
    }
    */

    ///Steps the whole voice-bank once, returning a modulated value based on "base_frequency". But everything is simd-ed.
    fn step_simd(&mut self, voice: usize, base_frequency: f32, sample_delta: f32) -> f32 {
        //we have two stepping procedures. One is the "high resolution"
        // phase.cos() for base osciis, and the lower resolution LFO type cos-less approximation.
        // TODO: implement https://www.cl.cam.ac.uk/~am21/hakmemc.html @ 151
        #[cfg(feature = "profile")]
        puffin::profile_function!();

        // we basically iterate over all ocs's here
        // and advance the oscillator's phase based on its current configuration
        // and the given `sample_delta`.
        //
        // However, since we want to SIMD this its a little bit uglier.
        // We still iterate over all, but only collect which osc's need stepping.
        // Whenever we fill a full simd lane we execute
        // it as well.
        //
        // Since we have have two types of OSC (Primary and Modulator) we also collect both types. The modulator functions differently
        // based on the current modulation type, but thats uniform over all, so we don't have to swizzle that out.

        let mut count;
        let mut accum = 0.0;
        let mut local_bases = simd::f32x4::splat(0.0);
        let mut local_multiplier = simd::f32x4::splat(1.0);
        let mut local_current_phase = simd::f32x4::splat(0.0);
        let mut local_volumes = simd::f32x4::splat(0.0);
        let mut local_phase_offsets = simd::f32x4::splat(0.0);

        assert!(Self::PRIMARY_OSC_COUNT % 4 == 0);
        assert!(Self::MOD_OSC_COUNT % 4 == 0);

        //phase step modulators, and upate parens's (possibly primary) oscillators
        // modulation value.
        // TODO: If the modulation strategy is "Absolute" we could
        //       Do the phase stepping for the whole bank in one pass instead of "per-voice"
        for lane_idx in 0..(Self::MOD_OSC_COUNT / 4) {
            let offset = lane_idx * 4;
            match self.mod_ty {
                ModulationType::Absolute => {
                    //for absolute modulation we use the ABS_BASE_FREQ for modulation offset, which is the same for all.
                    // This works similarly to the absolute one, but our base frequency is a static
                    // one instead of a voice based one.
                    for i in 0..4 {
                        let idx = Self::modulator_osc_index(voice, offset + i);
                        let osc = &mut self.modulator_osc[idx];
                        local_bases[i] = osc.osc.freq(Orbital::ABS_BASE_FREQ).max(0.0);
                        local_multiplier[i] = osc.freq_multiplier();
                        local_current_phase[i] = osc.phase;
                        local_phase_offsets[i] = osc.offset;
                    }
                }
                ModulationType::Relative => {
                    //At relative we use the voice's base frequency for
                    // and modulate that relatively.
                    //
                    // This is basically the same as the primary step below, but we are writing the result back to the
                    // parents instead
                    for i in 0..4 {
                        let idx = Self::modulator_osc_index(voice, offset + i);
                        let osc = &mut self.modulator_osc[idx];
                        local_bases[i] = osc.osc.freq(base_frequency).max(0.0);
                        local_multiplier[i] = osc.freq_multiplier();
                        local_current_phase[i] = osc.phase;
                        local_phase_offsets[i] = osc.offset;
                    }
                }
            }

            //after loading, do the phase step
            let result = Self::phase_step(
                local_bases,
                local_multiplier,
                local_current_phase,
                sample_delta,
            );

            //Write back the new phase and reset the modulation values for all. Those will be re-written in the step
            // below
            for i in 0..4 {
                let idx = Self::modulator_osc_index(voice, offset + i);
                let osc = &mut self.modulator_osc[idx];

                osc.phase = result[i];
                osc.mod_counter = 0;
                osc.mod_multiplier = 0.0;
            }
        }

        //We now have the updated modulators, therefore, we can iterate through all modulators
        // and update the parent's multiplier value.
        // Note that we can't do that in the first loop, since not all modulators might have stepped their phase yet,
        // which would produce a messy sampling.
        for lane_idx in 0..(Self::MOD_OSC_COUNT / 4) {
            let offset = lane_idx * 4;
            for i in 0..4 {
                let idx = Self::modulator_osc_index(voice, offset + i);
                let osc = &mut self.modulator_osc[idx];

                local_current_phase[i] = osc.phase;
                local_phase_offsets[i] = osc.offset;
                local_volumes[i] = osc.osc.range;
                if !osc.osc.is_on {
                    local_volumes[i] = 0.0;
                }
            }

            //Now evaluate the modulation values
            //NOTE: we got a phase for the mod oscillator. However the cos is (-1 .. 1). So we weight by range into (-range .. range).
            //      Next we want to only modulate the range around (100% - range .. 100% + range), so we add 1
            let modulation_samples = simd::f32x4::splat(1.0)
                + Self::simd_sample(local_current_phase, local_phase_offsets, local_volumes);

            //now write the modulation valuse to the parents
            for i in 0..4 {
                let idx = Self::modulator_osc_index(voice, offset + i);
                let osc = &self.modulator_osc[idx];

                //only write to parent osc if osc is actually on
                if osc.osc.is_on {
                    match osc.osc.parent_osc_slot {
                        ParentIndex::Modulator(modid) => {
                            let mod_osc =
                                &mut self.modulator_osc[Self::modulator_osc_index(voice, modid)];
                            mod_osc.mod_multiplier += modulation_samples[i];
                            mod_osc.mod_counter += 1;
                        }
                        ParentIndex::Primary(modid) => {
                            let prim_osc =
                                &mut self.primary_osc[Self::primary_osc_index(voice, modid)];
                            prim_osc.mod_multiplier += modulation_samples[i];
                            prim_osc.mod_counter += 1;
                        }
                    }
                }
            }
        }

        //Phase step primary oscillators and accumulate final, modulated
        // sample based on the evaluated `mod_multiplier` and `mod_counter`
        for lane_index in 0..(Self::PRIMARY_OSC_COUNT / 4) {
            #[cfg(feature = "profile")]
            puffin::profile_scope!("Primary phase step");

            let offset = lane_index * 4;
            count = 0;
            //fill primray oscillators into simd lanes
            for i in 0..4 {
                let idx = Self::primary_osc_index(voice, offset + i);
                let osc = &mut self.primary_osc[idx];

                local_bases[i] = osc.osc.freq(base_frequency).max(0.0);
                local_multiplier[i] = osc.freq_multiplier();
                local_current_phase[i] = osc.phase;
                local_phase_offsets[i] = osc.offset;
                local_volumes[i] = osc.osc.volume;

                if osc.osc.is_on {
                    //increase count for correct divisor
                    count += 1;
                } else {
                    local_volumes[i] = 0.0;
                }
            }

            //calculate lane results
            let result = Self::phase_step(
                local_bases,
                local_multiplier,
                local_current_phase,
                sample_delta,
            );

            //calculate accumulated samples
            if count > 0 {
                accum +=
                    Self::primary_sample(result, local_phase_offsets, local_volumes) / count as f32;
            }
            //write phase results to osc's and reset modulator
            for i in 0..4 {
                let idx = Self::primary_osc_index(voice, offset + i);
                let mut osc = &mut self.primary_osc[idx];
                osc.phase = result[i];
                osc.mod_counter = 0;
                osc.mod_multiplier = 1.0;
            }
        }

        accum
    }

    //Fills the buffer with sound jo
    pub fn process(
        &mut self,
        voices: &[OscVoiceState; OscillatorBank::VOICE_COUNT],
        buffer: &mut Buffer,
        sample_rate: f32,
        buffer_time_start: Time,
    ) {
        //PERFORMANCE:
        // Currently taking an max-avg of 8ms
        // all phase step: 2ms
        // all merge:      4ms
        // + overhead
        //
        // Singel voice: Bank: 12.8ms
        //               step: 3ms

        let delta_sec = (1.0 / sample_rate) as Time;

        #[cfg(feature = "profile")]
        {
            let num_voices = voices
                .iter()
                .fold(0, |f, v| if !v.state.is_off() { f + 1 } else { f });

            puffin::profile_function!(format!(
                "OSC-Bank[{} @ {}] process Max: {:.2}ms",
                num_voices,
                buffer.samples(),
                (buffer.samples() as f64 * delta_sec) * 1000.0
            ));
        }
        let mut sample_time = buffer_time_start;

        for mut sample in buffer.iter_samples() {
            let mut acc = 0.0;
            for vidx in 0..Self::VOICE_COUNT {
                if voices[vidx].state.is_off() {
                    continue;
                }
                let volume = voices[vidx].env.sample(sample_time);
                acc += self.step_simd(vidx, voices[vidx].freq, delta_sec as f32) * volume as f32;
            }

            let val = self.gain_ty.map(acc);
            for csam in sample.iter_mut() {
                *csam = val;
            }

            sample_time += delta_sec;
        }
    }
}
