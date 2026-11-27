// Appended inside the private module in the disposable replay build only.
impl O2Lane {
    pub fn state(&self) -> [f64; 4] {
        [self.v, self.h, self.z1, self.z2]
    }
    pub fn taps(&mut self, c: &O2Coefficients, v: f64, d: f64) -> [f64; 4] {
        self.advance(c, v, d).expect("valid replay sample")
    }
    pub fn fault_probe(&mut self, c: &O2Coefficients) {
        self.z1 = f64::MAX;
        let old = self.state().map(f64::to_bits);
        assert_eq!(self.tick(c, 0.75, 0.5), Err(OutputFault));
        assert_eq!(self.state().map(f64::to_bits), old, "partial publication");
    }
}
impl O2Coefficients {
    pub fn values(&self) -> [f64; 6] {
        [self.c, self.b0, self.b1, self.b2, self.a1, self.a2]
    }
}

pub(super) fn verify_faults(fs: f64) {
    let c = O2Coefficients::prepare(fs).unwrap();
    let mut lane = O2Lane::default();
    let zero = [0u64; 4];
    assert_eq!(lane.state().map(f64::to_bits), zero);
    println!("prepared-zero 4");
    for (name, v, d) in [
        ("invalid-input", 8.0_f64.next_up(), 0.5),
        ("invalid-detail", 1.0, f64::NAN),
    ] {
        let before = lane.state().map(f64::to_bits);
        assert_eq!(lane.tick(&c, v, d), Err(OutputFault));
        assert_eq!(lane.state().map(f64::to_bits), before);
        println!("{name} 1");
    }
    for i in 0..4 {
        lane.reset();
        *[&mut lane.v, &mut lane.h, &mut lane.z1, &mut lane.z2][i] = f64::NAN;
        let before = lane.state().map(f64::to_bits);
        assert_eq!(lane.tick(&c, 0.25, 0.5), Err(OutputFault));
        assert_eq!(lane.state().map(f64::to_bits), before);
    }
    println!("nonfinite-history 4");
    lane.reset();
    lane.fault_probe(&c);
    println!("prospective-overflow 1");
    for _ in 0..2 {
        lane.reset();
        assert_eq!(lane.state().map(f64::to_bits), zero, "dirty reset");
    }
    println!("repeated-dirty-reset 2");
    for invalid in [
        32000.0_f64.next_down(),
        192000.0_f64.next_up(),
        f64::NAN,
        f64::INFINITY,
    ] {
        assert!(O2Coefficients::prepare(invalid).is_err());
    }
    println!("invalid-preparation 4");
}
