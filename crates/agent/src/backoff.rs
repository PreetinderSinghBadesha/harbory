use std::time::Duration;

use rand::Rng;

const INITIAL: Duration = Duration::from_secs(1);
const MAX: Duration = Duration::from_secs(60);

/// Exponential backoff with +/-20% jitter, so a control-plane restart
/// doesn't get hammered by every agent reconnecting on the same schedule.
/// Resets to `INITIAL` after any successful connection.
pub struct Backoff {
    current: Duration,
}

impl Default for Backoff {
    fn default() -> Self {
        Self { current: INITIAL }
    }
}

impl Backoff {
    pub fn reset(&mut self) {
        self.current = INITIAL;
    }

    /// The delay to wait before the next attempt, with jitter applied.
    /// Also advances internal state so the *next* call returns a longer
    /// delay, up to `MAX`.
    pub fn next_delay(&mut self) -> Duration {
        let jitter_factor = rand::thread_rng().gen_range(0.8..1.2);
        let delay = self.current.mul_f64(jitter_factor);

        self.current = (self.current * 2).min(MAX);

        delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_near_initial_with_jitter() {
        let mut b = Backoff::default();
        let d = b.next_delay();
        assert!(d >= Duration::from_millis(800) && d <= Duration::from_millis(1200));
    }

    #[test]
    fn grows_and_caps_at_max() {
        let mut b = Backoff::default();
        for _ in 0..20 {
            let d = b.next_delay();
            assert!(d <= MAX.mul_f64(1.2));
        }
    }

    #[test]
    fn reset_returns_to_initial() {
        let mut b = Backoff::default();
        for _ in 0..5 {
            b.next_delay();
        }
        b.reset();
        let d = b.next_delay();
        assert!(d >= Duration::from_millis(800) && d <= Duration::from_millis(1200));
    }
}
