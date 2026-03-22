use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

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
