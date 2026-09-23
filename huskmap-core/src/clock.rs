use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Milliseconds since Unix epoch. Domain times are this, not `SystemTime`, so JSON is boring.
pub type Millis = u64;

pub fn millis_of(time: SystemTime) -> Millis {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
        Err(_) => 0,
    }
}

pub fn system_time_from_millis(ms: Millis) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(ms)
}

pub trait Clock: Send + Sync {
    fn now(&self) -> SystemTime;

    fn now_ms(&self) -> Millis {
        millis_of(self.now())
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

#[derive(Debug, Clone)]
pub struct FrozenClock {
    pub now: SystemTime,
}

impl FrozenClock {
    pub fn from_millis(ms: Millis) -> Self {
        Self {
            now: system_time_from_millis(ms),
        }
    }
}

impl Clock for FrozenClock {
    fn now(&self) -> SystemTime {
        self.now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_zero() {
        assert_eq!(millis_of(UNIX_EPOCH), 0);
    }

    #[test]
    fn before_epoch_is_zero() {
        let t = UNIX_EPOCH - Duration::from_secs(30);
        assert_eq!(millis_of(t), 0);
    }

    #[test]
    fn roundtrip() {
        let ms = 1_700_000_000_000;
        assert_eq!(millis_of(system_time_from_millis(ms)), ms);
    }

    #[test]
    fn frozen_clock() {
        let clock = FrozenClock::from_millis(42);
        assert_eq!(clock.now_ms(), 42);
    }

    #[test]
    fn system_clock_is_after_epoch() {
        assert!(SystemClock.now_ms() > 0);
    }

    #[test]
    fn overflow_millis_saturates() {
        let huge = UNIX_EPOCH + Duration::from_secs(u64::MAX / 2);
        let ms = millis_of(huge);
        assert!(ms > 0);
    }
}
