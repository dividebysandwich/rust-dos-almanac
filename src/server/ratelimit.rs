//! Per-address limits. Staff are never limited.

use super::config::Limits;
use super::error::Error;
use governor::clock::{Clock, DefaultClock};
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::time::Duration;

pub struct RateLimits {
    write_burst: DefaultKeyedRateLimiter<IpAddr>,
    write_daily: DefaultKeyedRateLimiter<IpAddr>,
    read: DefaultKeyedRateLimiter<IpAddr>,
    login: DefaultKeyedRateLimiter<IpAddr>,
    clock: DefaultClock,
}

fn nz(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n.max(1)).unwrap()
}

fn quota(burst: u32, period: Duration) -> Quota {
    Quota::with_period(period.max(Duration::from_millis(1))).expect("non-zero period").allow_burst(nz(burst))
}

impl RateLimits {
    pub fn new(l: &Limits) -> Self {
        let day = Duration::from_secs(86_400);
        Self {
            write_burst: RateLimiter::keyed(quota(l.write_burst, Duration::from_secs(l.write_refill_secs))),
            write_daily: RateLimiter::keyed(quota(l.writes_per_day, day / l.writes_per_day.max(1))),
            read: RateLimiter::keyed(Quota::per_minute(nz(l.reads_per_minute))),
            login: RateLimiter::keyed(quota(l.login_burst, Duration::from_secs(60))),
            clock: DefaultClock::default(),
        }
    }

    fn check(&self, limiter: &DefaultKeyedRateLimiter<IpAddr>, key: IpAddr) -> Result<(), Error> {
        limiter.check_key(&key).map_err(|not_until| {
            let wait = not_until.wait_time_from(self.clock.now());
            Error::RateLimited(wait.as_secs().max(1))
        })
    }

    /// One anonymous write. Both the burst and the daily budget must allow it.
    pub fn write(&self, key: IpAddr) -> Result<(), Error> {
        self.check(&self.write_burst, key)?;
        self.check(&self.write_daily, key)
    }

    pub fn read(&self, key: IpAddr) -> Result<(), Error> {
        self.check(&self.read, key)
    }

    pub fn login(&self, key: IpAddr) -> Result<(), Error> {
        self.check(&self.login, key)
    }

    /// Forget addresses that are back to a full budget.
    pub fn prune(&self) {
        for l in [&self.write_burst, &self.write_daily, &self.read, &self.login] {
            l.retain_recent();
            l.shrink_to_fit();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::Config;

    #[test]
    fn write_burst_then_429() {
        let limits = RateLimits::new(&Config::for_tests().limits);
        let a: IpAddr = "203.0.113.1".parse().unwrap();
        let b: IpAddr = "203.0.113.2".parse().unwrap();
        for _ in 0..5 {
            limits.write(a).unwrap();
        }
        assert!(matches!(limits.write(a), Err(Error::RateLimited(s)) if s > 0));
        limits.write(b).expect("other addresses are unaffected");
    }
}
