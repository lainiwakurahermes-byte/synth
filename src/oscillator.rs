/// A naive sawtooth oscillator.
///
/// `phase` is the position inside one cycle, in [0, 1). It is the whole state of the oscillator.
#[derive(Default)]
pub struct Oscillator {
    phase: f32,
}

impl Oscillator {
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    /// Returns one sample in [-1, 1] and advances the phase by one sample.
    pub fn next(&mut self, frequency: f32, sample_rate: f32) -> f32 {
        // The phase ramp already is a sawtooth; only rescale it from [0, 1) to [-1, 1).
        let sample = 2.0 * self.phase - 1.0;

        // One cycle takes sample_rate / frequency samples, so each sample covers this fraction.
        self.phase += frequency / sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        sample
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_completes_440_cycles_per_second() {
        let sample_rate = 48_000.0;
        let mut osc = Oscillator::default();

        let mut cycles = 0;
        let mut previous = osc.next(440.0, sample_rate);
        for _ in 1..48_000 {
            let sample = osc.next(440.0, sample_rate);
            assert!((-1.0..=1.0).contains(&sample));
            // The saw only ever falls when the phase wraps.
            if sample < previous {
                cycles += 1;
            }
            previous = sample;
        }

        assert!((439..=440).contains(&cycles), "got {cycles} cycles");
    }
}
