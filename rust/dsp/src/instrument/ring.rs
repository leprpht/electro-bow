//! M2 outgoing-wave history. The cursor points at the next write, so even
//! lag == capacity refers to committed history until the sample is committed.
pub(super) const BETA: f64 = 0.127;

#[derive(Clone, Copy)]
pub(super) enum Arm {
    Bridge,
    Nut,
}

#[derive(Clone, Copy)]
struct Seed {
    period: f64,
    amplitude: f64,
}

pub(super) struct DelayRing {
    samples: Box<[f64]>,
    rate: f64,
    arm: Arm,
    write: usize,
    count: usize,
    seed: Option<Seed>,
}

#[derive(Debug, PartialEq)]
pub(super) struct RingFault;

/// Validation has completed. Dropping the write leaves the epoch untouched.
#[must_use]
pub(super) struct PendingWrite<'a> {
    ring: &'a mut DelayRing,
    value: f64,
}

impl DelayRing {
    pub fn prepare(rate: f64, arm: Arm) -> Option<Self> {
        if !(32000.0..=192000.0).contains(&rate) {
            return None;
        }
        let capacity = (rate / 65.0).ceil() as usize + 4;
        let mut samples = Vec::new();
        samples.try_reserve_exact(capacity).ok()?;
        samples.resize(capacity, 0.0);
        Some(Self {
            samples: samples.into_boxed_slice(),
            rate,
            arm,
            write: 0,
            count: 0,
            seed: None,
        })
    }

    /// Open an M4 epoch under zero wet gain, keeping its original P,V on retune.
    pub fn launch(&mut self, hz: f64, speed: f64) -> Result<(), RingFault> {
        if !(65.0..=1320.0).contains(&hz) || !(0.06..=0.16).contains(&speed) {
            return Err(RingFault);
        }
        self.seed = Some(Seed {
            period: self.rate / hz,
            amplitude: speed / BETA,
        });
        self.write = 0;
        self.count = 0;
        Ok(())
    }

    /// Transport reset, completed release or fault. Old slots are unreachable.
    pub fn invalidate(&mut self) {
        self.seed = None;
        self.write = 0;
        self.count = 0;
    }

    pub fn read(&self, delay: f64) -> Result<f64, RingFault> {
        self.read_at(delay, 0)
    }

    /// For FIR preload, read the same tap at n-1 or n-2 without rounding D+s.
    pub fn read_at(&self, delay: f64, samples_ago: usize) -> Result<f64, RingFault> {
        let available = self.samples.len().saturating_sub(samples_ago);
        if !(1.0..=available as f64).contains(&delay) {
            return Err(RingFault);
        }
        let Some(seed) = self.seed else {
            return Ok(0.0);
        };
        let m = delay as usize;
        let u = delay - m as f64;
        let lag = m + samples_ago;
        let newer = self.history(lag, seed);
        if u == 0.0 {
            return Ok(newer);
        }
        Ok((1.0 - u) * newer + u * self.history(lag + 1, seed))
    }

    fn history(&self, lag: usize, seed: Seed) -> f64 {
        if lag <= self.count {
            let slot = if self.write >= lag {
                self.write - lag
            } else {
                self.samples.len() - (lag - self.write)
            };
            return self.samples[slot];
        }
        let t = -((lag - self.count) as f64) / seed.period;
        let (phase, amplitude) = match self.arm {
            Arm::Bridge => (t, seed.amplitude),
            Arm::Nut => (t + 1.0 - BETA, -seed.amplitude),
        };
        amplitude * (phase - phase.floor() - 0.5)
    }

    /// Stage both arms and finish all guide checks before committing either.
    pub fn stage(&mut self, value: f64) -> Result<PendingWrite<'_>, RingFault> {
        if self.seed.is_none() || !(-4.0..=4.0).contains(&value) {
            return Err(RingFault);
        }
        Ok(PendingWrite { ring: self, value })
    }
}

impl PendingWrite<'_> {
    pub fn commit(self) {
        let ring = self.ring;
        ring.samples[ring.write] = self.value;
        ring.write += 1;
        if ring.write == ring.samples.len() {
            ring.write = 0;
        }
        ring.count = (ring.count + 1).min(ring.samples.len());
    }
}

#[cfg(test)]
mod tests;
