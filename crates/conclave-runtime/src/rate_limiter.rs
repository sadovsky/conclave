/// Deterministic token bucket rate limiter.
///
/// All state is derived from virtual time and prior dispatch decisions.
/// No wall-clock access, no randomness.
pub struct TokenBucket {
    window_size_ms: u64,
    tokens_per_window: u32,
    current_window_start: u64,
    tokens_used_in_window: u32,
}

impl TokenBucket {
    pub fn new(window_size_ms: u64, tokens_per_window: u32) -> Self {
        TokenBucket {
            window_size_ms,
            tokens_per_window,
            current_window_start: 0,
            tokens_used_in_window: 0,
        }
    }

    /// Try to consume one token at virtual time `t`.
    ///
    /// Returns `Ok(())` if a token was available and consumed.
    /// Returns `Err(next_window_start)` if the window is exhausted.
    pub fn try_consume(&mut self, t: u64) -> Result<(), u64> {
        let window_start = self.window_for(t);
        if window_start != self.current_window_start {
            // New window — reset.
            self.current_window_start = window_start;
            self.tokens_used_in_window = 0;
        }

        if self.tokens_used_in_window < self.tokens_per_window {
            self.tokens_used_in_window += 1;
            Ok(())
        } else {
            Err(self.current_window_start + self.window_size_ms)
        }
    }

    /// The next window boundary after the current exhausted window.
    /// Returns `None` if the bucket still has tokens.
    pub fn next_window_start_if_exhausted(&self, t: u64) -> Option<u64> {
        let window_start = self.window_for(t);
        if window_start == self.current_window_start
            && self.tokens_used_in_window >= self.tokens_per_window
        {
            Some(self.current_window_start + self.window_size_ms)
        } else {
            None
        }
    }

    fn window_for(&self, t: u64) -> u64 {
        (t / self.window_size_ms) * self.window_size_ms
    }
}
