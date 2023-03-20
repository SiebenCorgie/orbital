use serde::{Deserialize, Serialize};

use crate::Time;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct EnvelopeParams {
    pub delay: Time,
    pub attack: Time,
    pub hold: Time,
    pub decay: Time,
    pub sustain_level: f32,
    pub release: Time,
}

impl Default for EnvelopeParams {
    fn default() -> Self {
        EnvelopeParams {
            delay: 0.0,
            attack: 0.2,
            hold: 0.1,
            decay: 0.1,
            sustain_level: 0.8,
            release: 0.1,
        }
    }
}

///Simple 5 stage envelope implementation. There are three state changing functions (via set), and a sample function.
/// Note that usually the parameters and values are in seconds, but in theory you can use anything.
///
///
/// A typical envelope lifetime. Note that you can set parts to 0 to remove them
/// ```skip
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
pub struct Envelope {
    pub press: Option<Time>,
    pub release: Option<Time>,
    pub parameters: EnvelopeParams,
}

impl Default for Envelope {
    fn default() -> Self {
        Envelope {
            press: None,
            release: None,
            parameters: EnvelopeParams::default(),
        }
    }
}

impl Envelope {
    ///sets the press event `at` the given time, resets the release event.
    pub fn on_press(&mut self, at: Time) {
        self.press = Some(at);
        self.release = None;
    }

    ///Sets release event `at` the given time. From now on if you sample after `at` you'll be in the release region.
    pub fn on_release(&mut self, at: Time) {
        self.release = Some(at);
    }

    pub fn reset(&mut self) {
        self.press = None;
        self.release = None;
    }
    //steps the delay-attack-hold-decay chain until `at`. If at too big sustain is returned, if to small,
    // 0.0 is returned
    fn step_linear(&self, at: Time) -> f32 {
        let start = if let Some(s) = self.press {
            s
        } else {
            return 0.0;
        };

        let mut local = at - start;
        //short path to decay
        if local
            > (self.parameters.delay
                + self.parameters.attack
                + self.parameters.hold
                + self.parameters.decay)
        {
            return self.parameters.sustain_level;
        }

        //also handles sub 0.0 local value
        if local < self.parameters.delay {
            return 0.0;
        } else {
            local -= self.parameters.delay;
        }

        //if here, we are in attack probably
        if local < self.parameters.attack {
            let alpha = ((local / self.parameters.attack) as f32).clamp(0.0, 1.0);
            return lerp(0.0, 1.0, alpha);
        } else {
            local -= self.parameters.attack;
        }

        //hat this point we are in hold
        if local < self.parameters.hold {
            return 1.0;
        } else {
            local -= self.parameters.hold;
        }

        //going into decay
        if local < self.parameters.decay {
            let alpha = ((local / self.parameters.decay) as f32).clamp(0.0, 1.0);
            return lerp(1.0, self.parameters.sustain_level, alpha);
        }

        //if not even here, we are actually in sustain
        self.parameters.sustain_level
    }

    pub fn after_sampling(&self, at: Time) -> bool {
        if let Some(end) = self.release {
            (end + self.parameters.release) < at
        } else {
            false
        }
    }

    ///samples a value of the current envelope. Note that the parameters are stacking.
    /// That means if `attack=1` and `delay=0` and `at=0.5` you'll get an attack value 0..1. If `delay=1` you'll get 0,
    /// since `at` is still in the decay range at that point.
    ///
    /// Note if no press event is set this will always return zero. But consider checking that case in your synth.
    pub fn sample(&self, at: Time) -> f32 {
        if self.press.is_none() {
            return 0.0;
        }

        if let Some(release) = self.release {
            //check where in release we are
            let relo = at - release;
            if relo < 0.0 {
                //not yet released, can happen at offsetted midi events
                self.step_linear(at)
            } else {
                if relo > self.parameters.release {
                    0.0
                } else {
                    //in release part
                    //check value at release, then interpolate to 0.0
                    let at_release = self.step_linear(release);
                    let normalize = ((relo / self.parameters.release) as f32).clamp(0.0, 1.0);
                    lerp(at_release, 0.0, normalize)
                }
            }
        } else {
            //calc linearly walked
            self.step_linear(at)
        }
    }
}

pub fn lerp(a: f32, b: f32, alpha: f32) -> f32 {
    (b * alpha) + (a * (1.0 - alpha))
}
