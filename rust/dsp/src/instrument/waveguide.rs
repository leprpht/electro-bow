//! One M2–M6 lane. The caller owns pitch and gesture trajectories and wet gain.
use super::friction::{ContactInput, ContactResult, Direction, FrictionCurve, Mode};
use super::ring::{Arm, BETA, DelayRing, RingFault};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Fault;

impl From<RingFault> for Fault {
    fn from(_: RingFault) -> Self {
        Self
    }
}

/// Velocities in m/s. Audio is the M5 DC-blocked pickup at unit captured gain.
#[derive(Debug, Clone, Copy)]
pub(super) struct Sample {
    pub from_nut: f64,
    pub from_bridge: f64,
    pub x: f64,
    pub pickup: f64,
    pub contact: ContactResult,
    pub audio: f64,
}

/// Both taps carry signed velocity in the outgoing nut-wave convention.
#[derive(Debug, Clone, Copy)]
pub(super) struct Station {
    pub direct: f64,
    pub reflected_history: f64,
}

impl Station {
    fn read(nut: &DelayRing, period: f64, position: f64) -> Self {
        Self {
            direct: nut.read_committed((position - BETA) * period / 2.0),
            reflected_history: nut.read_committed((2.0 - position - BETA) * period / 2.0),
        }
    }

    pub fn velocity(self) -> f64 {
        self.direct - self.reflected_history
    }
}

/// O1 observation in m/s at the same physical time as the returned M5 sample.
#[derive(Debug, Clone, Copy)]
pub(super) struct Observation {
    pub stations: [Station; 3],
}

impl Observation {
    fn read(nut: &DelayRing, period: f64) -> Self {
        let stations = [0.162, 0.174, 0.186].map(|s| Station::read(nut, period, s));
        Self { stations }
    }

    pub fn velocity(self) -> f64 {
        let [left, center, right] = self.stations.map(Station::velocity);
        (left + 4.0 * center + right) / 6.0
    }
}

pub(super) struct Guide {
    rate: f64,
    bridge: DelayRing,
    nut: DelayRing,
    curve: FrictionCurve,
    live: bool,
    loss: [f64; 2],
    mode: Mode,
    dc_coefficient: f64,
    previous_pickup: f64,
    previous_dc: f64,
    #[cfg(test)]
    edit_proposal: Option<fn(&mut Sample)>,
    #[cfg(test)]
    fault_count: usize,
    #[cfg(test)]
    launch_count: usize,
}

fn bounded(value: f64) -> bool {
    (-4.0..=4.0).contains(&value)
}

impl Guide {
    /// Allocate only during off-thread preparation. Destruction is off-thread.
    pub fn prepare(rate: f64) -> Option<Self> {
        Some(Self {
            rate,
            bridge: DelayRing::prepare(rate, Arm::Bridge)?,
            nut: DelayRing::prepare(rate, Arm::Nut)?,
            curve: FrictionCurve::prepare(),
            live: false,
            loss: [0.0; 2],
            mode: Mode::Stick,
            dc_coefficient: (-std::f64::consts::TAU * 20.0 / rate).exp(),
            previous_pickup: 0.0,
            previous_dc: 0.0,
            #[cfg(test)]
            edit_proposal: None,
            #[cfg(test)]
            fault_count: 0,
            #[cfg(test)]
            launch_count: 0,
        })
    }

    pub fn invalidate(&mut self) {
        self.live = false;
        self.bridge.invalidate();
        self.nut.invalidate();
        self.loss = [0.0; 2];
        self.mode = Mode::Stick;
        self.previous_pickup = 0.0;
        self.previous_dc = 0.0;
    }

    /// Confirmed launch or replacement, under zero wet gain.
    pub fn launch(&mut self, hz: f64, speed: f64) -> Result<(), Fault> {
        self.invalidate();
        let result = self.seed(hz, speed);
        if result.is_err() {
            self.fault();
        }
        result
    }

    fn fault(&mut self) {
        self.invalidate();
        #[cfg(test)]
        {
            self.fault_count += 1;
        }
    }

    fn seed(&mut self, hz: f64, speed: f64) -> Result<(), Fault> {
        self.bridge.launch(hz, speed)?;
        self.nut.launch(hz, speed)?;
        let delay = BETA * (self.rate / hz) - 1.0;
        let loss = [
            self.bridge.read_at(delay, 1)?,
            self.bridge.read_at(delay, 2)?,
        ];
        if !loss.into_iter().all(bounded) {
            return Err(Fault);
        }
        self.loss = loss;
        self.mode = Mode::Slide(Direction::Positive);
        self.live = true;
        #[cfg(test)]
        {
            self.launch_count += 1;
        }
        Ok(())
    }

    /// Ok(None) is a silent cleared lane. Err(Fault) starts controller recovery
    /// on this very sample. A lifted live lane continues to propagate.
    pub fn tick(&mut self, hz: f64, speed: f64, k: f64) -> Result<Option<Sample>, Fault> {
        Ok(self.advance(hz, speed, k)?.map(|(sample, _)| sample))
    }

    /// Advance once and capture O1 from this commit's nut history and period.
    /// Cleared and faulted ticks carry no observation.
    pub fn tick_observed(
        &mut self,
        hz: f64,
        speed: f64,
        k: f64,
    ) -> Result<Option<(Sample, Observation)>, Fault> {
        Ok(self
            .advance(hz, speed, k)?
            .map(|(sample, period)| (sample, Observation::read(&self.nut, period))))
    }

    fn advance(&mut self, hz: f64, speed: f64, k: f64) -> Result<Option<(Sample, f64)>, Fault> {
        let result = self.transaction(hz, speed, k);
        if result.is_err() {
            self.fault();
        }
        result
    }

    fn transaction(&mut self, hz: f64, speed: f64, k: f64) -> Result<Option<(Sample, f64)>, Fault> {
        if !(65.0..=1320.0).contains(&hz) || !(0.0..=0.16).contains(&speed) {
            return Err(Fault);
        }
        if k != 0.0
            && (!self.live
                || speed < 0.06
                || !(1.2 * speed / 1.65..=2.4 * speed / 1.65).contains(&k))
        {
            return Err(Fault);
        }
        if !self.live {
            return Ok(None);
        }
        if !self.loss.into_iter().all(bounded)
            || !bounded(self.previous_pickup)
            || !self.previous_dc.is_finite()
        {
            return Err(Fault);
        }

        let period = self.rate / hz;
        let bridge_distance = BETA * period;
        let x = self.bridge.read(bridge_distance - 1.0)?;
        let from_nut = -self.nut.read((1.0 - BETA) * period)?;
        let from_bridge = -0.98 * (0.125 * x + 0.75 * self.loss[0] + 0.125 * self.loss[1]);
        let pickup = self.bridge.read(bridge_distance / 2.0)?;
        let contact = self
            .curve
            .solve(
                ContactInput {
                    bow_speed: speed,
                    from_nut,
                    from_bridge,
                    k,
                },
                self.mode,
            )
            .map_err(|_| Fault)?;
        let dc = self.dc_coefficient * (self.previous_dc + pickup - self.previous_pickup);
        let sample = Sample {
            x,
            from_nut,
            from_bridge,
            contact,
            pickup,
            audio: 0.85 * dc,
        };
        #[cfg(test)]
        let sample = {
            let mut proposal = sample;
            if let Some(edit) = self.edit_proposal {
                edit(&mut proposal);
            }
            proposal
        };
        if ![
            sample.x,
            sample.from_nut,
            sample.from_bridge,
            sample.pickup,
            sample.contact.j,
        ]
        .into_iter()
        .all(bounded)
            || ![
                sample.contact.slip,
                sample.contact.force,
                sample.contact.string_velocity,
                sample.audio,
            ]
            .into_iter()
            .all(f64::is_finite)
        {
            return Err(Fault);
        }

        let bridge = self.bridge.stage(sample.contact.to_bridge)?;
        let nut = self.nut.stage(sample.contact.to_nut)?;
        bridge.commit();
        nut.commit();
        self.loss = [sample.x, self.loss[0]];
        self.mode = sample.contact.mode;
        self.previous_pickup = sample.pickup;
        self.previous_dc = dc;
        Ok(Some((sample, period)))
    }
}

#[cfg(test)]
mod tests;
