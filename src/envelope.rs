/// The four envelope controls. Times are in seconds, sustain is a level in [0, 1].
///
/// Each time is how long a full sweep between 0 and 1 takes. A release from a sustain of 0.5
/// therefore takes half the release time.
pub struct EnvelopeSettings {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

#[derive(Default, PartialEq)]
enum Stage {
    /// Silent, waiting for a key.
    #[default]
    Idle,
    /// Key down: rise to full level.
    Attack,
    /// Key still down: move to the sustain level and stay there.
    DecaySustain,
    /// Key up: fall to silence.
    Release,
}

/// An ADSR envelope with straight-line segments. It outputs a gain in [0, 1].
#[derive(Default)]
pub struct Envelope {
    stage: Stage,
    level: f32,
}

impl Envelope {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Key down. The attack starts from the current level, so a retrigger cannot click.
    pub fn gate_on(&mut self) {
        self.stage = Stage::Attack;
    }

    /// Key up.
    pub fn gate_off(&mut self) {
        if self.stage != Stage::Idle {
            self.stage = Stage::Release;
        }
    }

    /// Returns the gain for one sample and advances the envelope.
    pub fn next(&mut self, settings: &EnvelopeSettings, sample_rate: f32) -> f32 {
        // How far the level moves in one sample for a segment of the given length.
        let step = |seconds: f32| 1.0 / (seconds * sample_rate);

        match self.stage {
            Stage::Idle => (),
            Stage::Attack => {
                self.level += step(settings.attack);
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::DecaySustain;
                }
            }
            Stage::DecaySustain => {
                // Chasing the sustain level in both directions also glides when the knob moves.
                let step = step(settings.decay);
                self.level += (settings.sustain - self.level).clamp(-step, step);
            }
            Stage::Release => {
                self.level -= step(settings.release);
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }

        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 1_000.0;
    const SETTINGS: EnvelopeSettings = EnvelopeSettings {
        attack: 0.1,
        decay: 0.2,
        sustain: 0.5,
        release: 0.4,
    };

    fn run(envelope: &mut Envelope, samples: usize) -> f32 {
        let mut level = 0.0;
        for _ in 0..samples {
            level = envelope.next(&SETTINGS, SAMPLE_RATE);
        }
        level
    }

    #[test]
    fn follows_attack_decay_sustain_release() {
        let mut envelope = Envelope::default();
        assert_eq!(run(&mut envelope, 10), 0.0, "silent before the key");

        envelope.gate_on();
        assert!((run(&mut envelope, 50) - 0.5).abs() < 0.01, "halfway up the attack");
        assert!((run(&mut envelope, 51) - 1.0).abs() < 0.01, "attack peak");
        // The decay covers 1.0 -> 0.5, half a full sweep, so it takes half the decay time.
        assert!((run(&mut envelope, 101) - 0.5).abs() < 0.01, "sustain reached");
        assert!((run(&mut envelope, 500) - 0.5).abs() < 0.01, "sustain holds");

        envelope.gate_off();
        assert!((run(&mut envelope, 100) - 0.25).abs() < 0.01, "halfway down the release");
        assert_eq!(run(&mut envelope, 110), 0.0, "silent after the release");
    }

    #[test]
    fn never_jumps() {
        let mut envelope = Envelope::default();
        let mut previous = 0.0;
        for n in 0..2_000 {
            match n {
                100 | 130 | 900 => envelope.gate_on(),
                120 | 600 | 1_000 => envelope.gate_off(),
                _ => (),
            }
            let level = envelope.next(&SETTINGS, SAMPLE_RATE);
            // The steepest segment is the attack: 1 / (0.1 s * 1000 Hz) per sample.
            assert!((level - previous).abs() <= 0.0101, "jump at sample {n}");
            previous = level;
        }
    }
}
