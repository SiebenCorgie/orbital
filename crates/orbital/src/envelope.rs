use nih_plug::prelude::Params;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct EnvelopeParams{
    delay: f32,
    attack: f32,
    hold: f32,
    decay: f32,
    sustain_level: f32,
    release: f32
}

impl Default for EnvelopeParams{
    fn default() -> Self {
        EnvelopeParams { delay: 0.0, attack: 0.1, hold: 1.0, decay: 1.0, sustain_level: 0.9, release: 1.0 }
    }
}

///Simple 5 stage envelope implementation. There are three state changing functions (via set), and a sample function.
/// Note that usually the parameters and values are in seconds, but in theory you can use anything.
///
///
/// A typical envelope lifetime. Note that you can set parts to 0 to remove them
/// ```
/// sampled value
/// 1^
///  |          /--------\__
///  |         /            \____
///  |        /                 \
///  |       /                   \
///  |      /                     \
///  |     /                       \
///  +------------------------------> time
///
///  |delay|attack| hold | decay | release
///
///  ^                           ^
///  | press event               | release event
/// ```
pub struct Envelope{
    pub press: Option<f32>,
    pub release: Option<f32>,
    pub parameters: EnvelopeParams,
}

impl Default for Envelope{
    fn default() -> Self {
        Envelope { press: None, release: None, parameters: EnvelopeParams::default() }
    }
}

impl Envelope{
    ///sets the press event `at` the given time, resets the release event.
    pub fn on_press(&mut self, at: f32){

    }

    ///Sets release event `at` the given time. From now on if you sample after `at` you'll be in the release region.
    pub fn on_release(&mut self, at: f32){

    }

    ///samples a value of the current envelope. Note that the parameters are stacking.
    /// That means if `attack=1` and `delay=0` and `at=0.5` you'll get an attack value 0..1. If `delay=1` you'll get 0,
    /// since `at` is still in the decay range at that point.
    ///
    /// Note if no press event is set this will always return zero. But consider checking that case in your synth.
    pub fn sample(&self, mut at: f32) -> f32{
        //the implementation actually "walks" through the stages until it
        // is within a stage or exceeding. In the latter case we return 0 is release was set, or
        // the sustain value.

        let start = if let Some(s) = self.press{
            s
        }else{
            return 0.0;
        };

        //offset into "local" time, where start == 0;
        at = at - start;

        //check delay, if within just return zero.
        if at < self.parameters.delay{
            return 0.0;
        }else{
            at -= self.parameters.delay;
        }

        //at attack, interpolate where on attack we are
        if at < self.parameters.attack{
            //calculate where on attack we are. Our attack is always 0..1, so this is simple interpolation
            return at / self.parameters.attack;
        }else {
            at -= self.parameters.attack;
        }

        //on hold simply return 1.0
        if at < self.parameters.hold{
            return 1.0;
        }else{
            at -= self.parameters.hold;
        }

        //on decay interpolate between 1.0 and sustain level.
        if at < self.parameters.decay{
            let perc = at / self.parameters.decay;
            return lerp(1.0, self.parameters.sustain_level, perc);
        }else{
            at -= self.parameters.decay;
        }

        //this is the jucy part. Basically, if a release was set, advance into the release
        // part (like we did above), either interpolate, or return
        if let Some(release) = self.release{
            //offset into release
            at -= release;
            if at < self.parameters.release{
                let perc = at / self.parameters.release;
                lerp(self.parameters.sustain_level, 0.0, perc)
            }else{
                //not even in release, therefore off
                0.0
            }
        }else{
            self.parameters.sustain_level
        }
    }
}


fn lerp(a: f32, b: f32, alpha: f32) -> f32{
    (a * alpha) + (b * (1.0-alpha))
}
