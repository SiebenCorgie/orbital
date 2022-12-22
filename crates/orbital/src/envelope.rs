use serde::{Serialize, Deserialize};

use crate::Time;

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct EnvelopeParams{
    delay: Time,
    attack: Time,
    hold: Time,
    decay: Time,
    sustain_level: f32,
    release: Time
}

impl Default for EnvelopeParams{
    fn default() -> Self {
        EnvelopeParams { delay: 0.0, attack: 0.1, hold: 0.1, decay: 0.1, sustain_level: 0.5, release: 0.1 }
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
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Envelope{
    pub press: Option<Time>,
    pub release: Option<Time>,
    pub parameters: EnvelopeParams,
}

impl Default for Envelope{
    fn default() -> Self {
        Envelope {
            press: None,
            release: None,
            parameters: EnvelopeParams::default(),
        }
    }
}

impl Envelope{
    ///sets the press event `at` the given time, resets the release event.
    pub fn on_press(&mut self, at: Time){
        self.press = Some(at);
        self.release = None;
    }

    ///Sets release event `at` the given time. From now on if you sample after `at` you'll be in the release region.
    pub fn on_release(&mut self, at: Time){
        self.release = Some(at);
    }

    //walks the linear part until "at".
    fn walk_to_linear(&self, mut at: Time) -> f32{
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

        //check delay, if within just return zero. This also applies for instance if
        // start is in the future.
        if at < self.parameters.delay{
            return 0.0;
        }else{
            at -= self.parameters.delay;
        }

        //at attack, interpolate where on attack we are
        if at < self.parameters.attack{
            //calculate where on attack we are. Our attack is always 0..1, so this is simple interpolation
            let alpha = at / self.parameters.attack;
            return lerp(0.0, 1.0, alpha as f32);
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
            lerp(1.0, self.parameters.sustain_level, perc as f32)
        }else{
            self.parameters.sustain_level as f32
        }
    }

    ///samples a value of the current envelope. Note that the parameters are stacking.
    /// That means if `attack=1` and `delay=0` and `at=0.5` you'll get an attack value 0..1. If `delay=1` you'll get 0,
    /// since `at` is still in the decay range at that point.
    ///
    /// Note if no press event is set this will always return zero. But consider checking that case in your synth.
    pub fn sample(&self, at: Time) -> f32{


        //if we have a release event we have to check if we are actually in the release part, if so,
        // do not walk to "at", but the release event, then interpolate the "last" value to zero
        if let Some(release_at) = self.release{
            let release_part = at - release_at;
            if release_part > 0.0{
                //in decent
                if release_part < self.parameters.release{
                    let val_at_release = self.walk_to_linear(release_at);
                    let normalized = release_part / self.parameters.release;
                    lerp(val_at_release, 0.0, normalized as f32) as f32
                }else{
                    //already out of decent
                    0.0
                }
            }else{
                //still in "normal" walk
                self.walk_to_linear(at)
            }
        }else{
            self.walk_to_linear(at)
        }
    }
}


pub fn lerp(a: f32, b: f32, alpha: f32) -> f32{
    (b * alpha) + (a * (1.0-alpha))
}
