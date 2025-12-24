use rand::Rng;
use std::time::Duration;
use tracing::debug;

use crate::config::TarpitConfig;

/// Session-based tarpit for wasting attacker time with progressive delays
#[derive(Debug, Clone)]
pub struct Tarpit {
    config: TarpitConfig,
    /// Number of interactions in this session
    interaction_count: u32,
    /// Total delay applied in this session (milliseconds)
    total_delay_ms: u64,
}

impl Tarpit {
    /// Create a new Tarpit instance for a session
    pub fn new(config: TarpitConfig) -> Self {
        Self {
            config,
            interaction_count: 0,
            total_delay_ms: 0,
        }
    }

    /// Check if tarpit is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the delay for the next response, incrementing the interaction count
    pub fn get_delay(&mut self) -> Duration {
        if !self.config.enabled {
            return Duration::ZERO;
        }

        self.interaction_count += 1;

        // Calculate base delay with progressive multiplier
        let delay_ms = if self.config.progressive && self.interaction_count > 1 {
            let multiplier = self.config.delay_multiplier.powi((self.interaction_count - 1) as i32);
            (self.config.base_delay_ms as f64 * multiplier) as u64
        } else {
            self.config.base_delay_ms
        };

        // Cap at max delay
        let delay_ms = delay_ms.min(self.config.max_delay_ms);

        // Apply jitter
        let delay_ms = self.apply_jitter(delay_ms);

        self.total_delay_ms += delay_ms;

        debug!(
            "Tarpit delay: {}ms (interaction #{}, total wasted: {}ms)",
            delay_ms, self.interaction_count, self.total_delay_ms
        );

        Duration::from_millis(delay_ms)
    }

    /// Apply the tarpit delay asynchronously
    pub async fn apply_delay(&mut self) {
        let delay = self.get_delay();
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
    }

    /// Apply random jitter to a delay value
    fn apply_jitter(&self, delay_ms: u64) -> u64 {
        if self.config.jitter_percent == 0 {
            return delay_ms;
        }

        let jitter_range = (delay_ms as f64 * self.config.jitter_percent as f64 / 100.0) as u64;
        if jitter_range == 0 {
            return delay_ms;
        }

        let mut rng = rand::thread_rng();
        let jitter = rng.gen_range(0..=jitter_range * 2);

        // Jitter can be +/- the jitter_range
        delay_ms.saturating_sub(jitter_range).saturating_add(jitter)
    }

    /// Get the current interaction count
    pub fn interaction_count(&self) -> u32 {
        self.interaction_count
    }

    /// Get the total delay applied in this session (milliseconds)
    pub fn total_delay_ms(&self) -> u64 {
        self.total_delay_ms
    }

    /// Get a summary of the tarpit session for logging
    pub fn summary(&self) -> String {
        format!(
            "Tarpit: {} interactions, {}ms total delay",
            self.interaction_count, self.total_delay_ms
        )
    }
}

impl Default for Tarpit {
    fn default() -> Self {
        Self::new(TarpitConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_config() -> TarpitConfig {
        TarpitConfig {
            enabled: true,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            progressive: true,
            delay_multiplier: 1.5,
            jitter_percent: 0, // No jitter for predictable tests
        }
    }

    #[test]
    fn test_tarpit_disabled() {
        let mut tarpit = Tarpit::new(TarpitConfig::default());
        assert!(!tarpit.is_enabled());
        assert_eq!(tarpit.get_delay(), Duration::ZERO);
    }

    #[test]
    fn test_tarpit_enabled_base_delay() {
        let mut config = enabled_config();
        config.progressive = false;
        let mut tarpit = Tarpit::new(config);

        assert!(tarpit.is_enabled());
        assert_eq!(tarpit.get_delay(), Duration::from_millis(100));
        assert_eq!(tarpit.get_delay(), Duration::from_millis(100));
        assert_eq!(tarpit.interaction_count(), 2);
    }

    #[test]
    fn test_tarpit_progressive_delays() {
        let mut tarpit = Tarpit::new(enabled_config());

        // First interaction: base delay (100ms)
        assert_eq!(tarpit.get_delay(), Duration::from_millis(100));

        // Second interaction: 100 * 1.5 = 150ms
        assert_eq!(tarpit.get_delay(), Duration::from_millis(150));

        // Third interaction: 100 * 1.5^2 = 225ms
        assert_eq!(tarpit.get_delay(), Duration::from_millis(225));

        // Fourth interaction: 100 * 1.5^3 = 337ms
        assert_eq!(tarpit.get_delay(), Duration::from_millis(337));

        assert_eq!(tarpit.interaction_count(), 4);
        assert_eq!(tarpit.total_delay_ms(), 100 + 150 + 225 + 337);
    }

    #[test]
    fn test_tarpit_max_delay_cap() {
        let mut config = enabled_config();
        config.max_delay_ms = 200;
        let mut tarpit = Tarpit::new(config);

        // First: 100ms
        assert_eq!(tarpit.get_delay(), Duration::from_millis(100));
        // Second: 150ms
        assert_eq!(tarpit.get_delay(), Duration::from_millis(150));
        // Third: would be 225ms, capped to 200ms
        assert_eq!(tarpit.get_delay(), Duration::from_millis(200));
        // Fourth: would be 337ms, capped to 200ms
        assert_eq!(tarpit.get_delay(), Duration::from_millis(200));
    }

    #[test]
    fn test_tarpit_jitter() {
        let mut config = enabled_config();
        config.progressive = false;
        config.jitter_percent = 50;
        let mut tarpit = Tarpit::new(config);

        // With 50% jitter on 100ms, delay should be between 50-150ms
        for _ in 0..10 {
            let delay = tarpit.get_delay();
            assert!(delay >= Duration::from_millis(50));
            assert!(delay <= Duration::from_millis(150));
        }
    }

    #[test]
    fn test_tarpit_summary() {
        let mut tarpit = Tarpit::new(enabled_config());
        tarpit.get_delay();
        tarpit.get_delay();

        let summary = tarpit.summary();
        assert!(summary.contains("2 interactions"));
        assert!(summary.contains("250ms total delay"));
    }
}