/// A deterministic virtual clock measured in milliseconds.
///
/// Never wraps `Instant::now()` or `SystemTime::now()`.
/// Time only advances explicitly via `advance` or `advance_to`.
pub struct VirtualClock {
    t: u64,
}

impl VirtualClock {
    pub fn new() -> Self {
        VirtualClock { t: 0 }
    }

    /// Current virtual time in milliseconds.
    pub fn now(&self) -> u64 {
        self.t
    }

    /// Advance by `delta_ms` milliseconds.
    pub fn advance(&mut self, delta_ms: u64) {
        self.t += delta_ms;
    }

    /// Advance to an absolute virtual time.
    ///
    /// Panics if `t` is in the past — virtual time must never go backwards.
    pub fn advance_to(&mut self, t: u64) {
        assert!(
            t >= self.t,
            "virtual time must not go backwards: {t} < {}",
            self.t
        );
        self.t = t;
    }
}

impl Default for VirtualClock {
    fn default() -> Self {
        Self::new()
    }
}
