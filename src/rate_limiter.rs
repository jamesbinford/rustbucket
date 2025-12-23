use config::{Config, File};
use rand::Rng;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;

/// Configuration for rate limiting behavior
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    /// Enable/disable rate limiting entirely
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Maximum concurrent connections per IP
    #[serde(default = "default_max_connections")]
    pub max_connections_per_ip: u32,
    /// Maximum new connections per IP per minute
    #[serde(default = "default_rate")]
    pub connection_rate_per_minute: u32,
    /// Number of connections before temporary ban
    #[serde(default = "default_ban_threshold")]
    pub ban_threshold: u32,
    /// Duration of temporary ban in seconds
    #[serde(default = "default_ban_duration")]
    pub ban_duration_seconds: u64,
    /// Response delay range [min_ms, max_ms]
    #[serde(default = "default_delay")]
    pub response_delay_ms: (u64, u64),
    /// IPs that bypass rate limiting entirely
    #[serde(default)]
    pub allowlist: Vec<String>,
    /// IPs that are always blocked
    #[serde(default)]
    pub blocklist: Vec<String>,
}

fn default_enabled() -> bool { true }
fn default_max_connections() -> u32 { 5 }
fn default_rate() -> u32 { 10 }
fn default_ban_threshold() -> u32 { 50 }
fn default_ban_duration() -> u64 { 3600 }
fn default_delay() -> (u64, u64) { (100, 300) }

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            max_connections_per_ip: default_max_connections(),
            connection_rate_per_minute: default_rate(),
            ban_threshold: default_ban_threshold(),
            ban_duration_seconds: default_ban_duration(),
            response_delay_ms: default_delay(),
            allowlist: Vec::new(),
            blocklist: Vec::new(),
        }
    }
}

/// Per-IP connection state
#[derive(Debug, Clone)]
pub struct IpState {
    /// Current active connections from this IP
    pub active_connections: u32,
    /// Timestamps of connections in last minute (for rate calculation)
    pub connection_timestamps: Vec<Instant>,
    /// Total connections from this IP (for ban threshold)
    pub total_connections: u32,
    /// When this IP was banned (if banned)
    pub banned_until: Option<Instant>,
    /// Last activity timestamp
    pub last_seen: Instant,
}

impl Default for IpState {
    fn default() -> Self {
        Self {
            active_connections: 0,
            connection_timestamps: Vec::new(),
            total_connections: 0,
            banned_until: None,
            last_seen: Instant::now(),
        }
    }
}

/// Rate limiter for connection management
pub struct RateLimiter {
    config: RateLimitConfig,
    /// Per-IP state tracking
    ip_states: RwLock<HashMap<IpAddr, IpState>>,
    /// Parsed allowlist
    allowlist: HashSet<IpAddr>,
    /// Parsed blocklist
    blocklist: HashSet<IpAddr>,
}

/// Shared reference type for passing to handlers
pub type RateLimiterRef = Arc<RateLimiter>;

impl RateLimiter {
    /// Create new RateLimiter, loading config from Config.toml
    pub fn new() -> Self {
        let config = Self::load_config();

        // Parse allowlist/blocklist IPs
        let allowlist: HashSet<IpAddr> = config.allowlist.iter()
            .filter_map(|s| s.parse().ok())
            .collect();
        let blocklist: HashSet<IpAddr> = config.blocklist.iter()
            .filter_map(|s| s.parse().ok())
            .collect();

        info!(
            "RateLimiter initialized: enabled={}, max_conn={}, rate={}/min, ban_threshold={}, delay={:?}ms",
            config.enabled,
            config.max_connections_per_ip,
            config.connection_rate_per_minute,
            config.ban_threshold,
            config.response_delay_ms
        );

        Self {
            config,
            ip_states: RwLock::new(HashMap::new()),
            allowlist,
            blocklist,
        }
    }

    fn load_config() -> RateLimitConfig {
        let settings = Config::builder()
            .add_source(File::with_name("Config").required(false))
            .build();

        match settings {
            Ok(s) => s.get("rate_limiting").unwrap_or_default(),
            Err(_) => RateLimitConfig::default(),
        }
    }

    /// Check if a connection from this IP should be allowed
    /// Returns Ok(()) if allowed, Err(reason) if blocked
    pub async fn check_connection(&self, ip: IpAddr) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }

        // Check blocklist first
        if self.blocklist.contains(&ip) {
            info!("Rate limit: {} is blocklisted", ip);
            return Err("IP is blocklisted".to_string());
        }

        // Allowlisted IPs bypass all checks
        if self.allowlist.contains(&ip) {
            return Ok(());
        }

        let mut states = self.ip_states.write().await;
        let state = states.entry(ip).or_insert_with(IpState::default);

        // Check if banned
        if let Some(banned_until) = state.banned_until {
            if Instant::now() < banned_until {
                let remaining = banned_until.duration_since(Instant::now());
                info!("Rate limit: {} is banned for {:?} more", ip, remaining);
                return Err("IP is temporarily banned".to_string());
            } else {
                // Ban expired
                state.banned_until = None;
                state.total_connections = 0; // Reset after ban expires
            }
        }

        // Check concurrent connection limit
        if state.active_connections >= self.config.max_connections_per_ip {
            info!("Rate limit: {} exceeded max concurrent connections ({})",
                ip, state.active_connections);
            return Err("Too many concurrent connections".to_string());
        }

        // Clean old timestamps and check rate
        let one_minute_ago = Instant::now() - Duration::from_secs(60);
        state.connection_timestamps.retain(|t| *t > one_minute_ago);

        if state.connection_timestamps.len() as u32 >= self.config.connection_rate_per_minute {
            state.total_connections += 1;

            // Check if should ban
            if state.total_connections >= self.config.ban_threshold {
                state.banned_until = Some(
                    Instant::now() + Duration::from_secs(self.config.ban_duration_seconds)
                );
                info!("Rate limit: {} banned for {} seconds (exceeded threshold)",
                    ip, self.config.ban_duration_seconds);
                return Err("IP banned due to excessive connections".to_string());
            }

            info!("Rate limit: {} exceeded rate limit ({}/min)",
                ip, self.config.connection_rate_per_minute);
            return Err("Connection rate exceeded".to_string());
        }

        // Record connection
        state.connection_timestamps.push(Instant::now());
        state.active_connections += 1;
        state.total_connections += 1;
        state.last_seen = Instant::now();

        Ok(())
    }

    /// Call when connection ends to decrement active count
    pub async fn release_connection(&self, ip: IpAddr) {
        if !self.config.enabled {
            return;
        }

        let mut states = self.ip_states.write().await;
        if let Some(state) = states.get_mut(&ip) {
            state.active_connections = state.active_connections.saturating_sub(1);
        }
    }

    /// Get random delay within configured range
    pub fn get_response_delay(&self) -> Duration {
        if !self.config.enabled {
            return Duration::ZERO;
        }

        let (min, max) = self.config.response_delay_ms;
        if min >= max {
            return Duration::from_millis(min);
        }

        let delay_ms = rand::thread_rng().gen_range(min..=max);
        Duration::from_millis(delay_ms)
    }

    /// Apply response delay (call before sending response)
    pub async fn apply_response_delay(&self) {
        let delay = self.get_response_delay();
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
    }

}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = RateLimitConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_connections_per_ip, 5);
        assert_eq!(config.connection_rate_per_minute, 10);
        assert_eq!(config.ban_threshold, 50);
        assert_eq!(config.ban_duration_seconds, 3600);
        assert_eq!(config.response_delay_ms, (100, 300));
    }

    #[test]
    fn test_ip_state_defaults() {
        let state = IpState::default();
        assert_eq!(state.active_connections, 0);
        assert_eq!(state.total_connections, 0);
        assert!(state.banned_until.is_none());
    }

    #[tokio::test]
    async fn test_rate_limiter_allows_connection() {
        let limiter = RateLimiter::new();
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // First connection should be allowed
        assert!(limiter.check_connection(ip).await.is_ok());

        // Cleanup
        limiter.release_connection(ip).await;
    }

    #[tokio::test]
    async fn test_rate_limiter_concurrent_limit() {
        let limiter = RateLimiter::new();
        let ip: IpAddr = "192.168.1.2".parse().unwrap();

        // Fill up to max connections (default 5)
        for _ in 0..5 {
            assert!(limiter.check_connection(ip).await.is_ok());
        }

        // 6th connection should be rejected
        assert!(limiter.check_connection(ip).await.is_err());

        // Release one and try again
        limiter.release_connection(ip).await;
        assert!(limiter.check_connection(ip).await.is_ok());
    }

    #[test]
    fn test_response_delay_range() {
        let limiter = RateLimiter::new();

        for _ in 0..10 {
            let delay = limiter.get_response_delay();
            assert!(delay >= Duration::from_millis(100));
            assert!(delay <= Duration::from_millis(300));
        }
    }
}
