// Appended only in the disposable validation build, inside the real Guide module.
impl Guide {
    pub(super) fn check_epoch(&self, ticks: usize, seed: (f64, f64)) {
        assert_eq!(
            self.launch_count, 1,
            "guide launch count after {ticks} ticks"
        );
        assert_eq!(self.fault_count, 0, "guide fault count after {ticks} ticks");
        assert_eq!(
            self.invalidations, 1,
            "unexpected invalidation after {ticks} ticks"
        );
        assert!(self.live, "lost epoch after {ticks} ticks");
        self.bridge.check_epoch(ticks, seed);
        self.nut.check_epoch(ticks, seed);
    }

    pub(super) fn lifecycle(&self) -> (usize, usize, usize) {
        (self.launch_count, self.fault_count, self.invalidations)
    }
}
