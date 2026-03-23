use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

const LOGIN_MAX_ATTEMPTS: u32 = 5;
const LOGIN_LOCKOUT: Duration = Duration::from_secs(15 * 60); // 15 minutes

struct LoginEntry {
    failures: u32,
    last_failure: Instant,
}

/// Tracks failed login attempts per IP and locks out after too many failures.
pub struct LoginRateLimiter {
    map: Mutex<HashMap<String, LoginEntry>>,
}

impl LoginRateLimiter {
    pub fn new() -> Self {
        Self { map: Mutex::new(HashMap::new()) }
    }

    /// Returns true if the IP is currently locked out.
    pub fn is_locked(&self, ip: &str) -> bool {
        let map = self.map.lock().unwrap();
        match map.get(ip) {
            Some(e) if e.failures >= LOGIN_MAX_ATTEMPTS => {
                Instant::now().duration_since(e.last_failure) < LOGIN_LOCKOUT
            }
            _ => false,
        }
    }

    /// Returns remaining lockout seconds (0 if not locked).
    pub fn lockout_secs_remaining(&self, ip: &str) -> u64 {
        let map = self.map.lock().unwrap();
        match map.get(ip) {
            Some(e) if e.failures >= LOGIN_MAX_ATTEMPTS => {
                let elapsed = Instant::now().duration_since(e.last_failure);
                LOGIN_LOCKOUT.saturating_sub(elapsed).as_secs()
            }
            _ => 0,
        }
    }

    /// Record a failed login attempt.
    pub fn record_failure(&self, ip: &str) {
        let mut map = self.map.lock().unwrap();
        let entry = map.entry(ip.to_string()).or_insert(LoginEntry {
            failures: 0,
            last_failure: Instant::now(),
        });
        entry.failures += 1;
        entry.last_failure = Instant::now();
    }

    /// Clear the failure record for this IP on successful login.
    pub fn record_success(&self, ip: &str) {
        self.map.lock().unwrap().remove(ip);
    }
}

pub struct RateLimiter {
    posts: Mutex<HashMap<String, Instant>>,
    cooldown: Duration,
}

impl RateLimiter {
    pub fn new(cooldown_secs: u64) -> Self {
        Self {
            posts: Mutex::new(HashMap::new()),
            cooldown: Duration::from_secs(cooldown_secs),
        }
    }

    /// Returns true if the request is allowed, false if rate limited.
    pub fn check_and_record(&self, key: &str) -> bool {
        let mut map = self.posts.lock().unwrap();
        let now = Instant::now();

        // Purge stale entries to prevent unbounded memory growth
        if map.len() > 10_000 {
            map.retain(|_, last| now.duration_since(*last) < self.cooldown * 10);
        }

        if let Some(&last) = map.get(key) {
            if now.duration_since(last) < self.cooldown {
                return false;
            }
        }

        map.insert(key.to_string(), now);
        true
    }
}
