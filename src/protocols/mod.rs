pub mod ssh;
pub mod ssh_shell;
pub mod http;
pub mod ftp;
pub mod smtp;

use tokio::io::{AsyncRead, AsyncWrite};
use std::time::{Duration, Instant};
use std::collections::HashSet;

/// Session state tracking for LLM escalation decisions
#[derive(Debug, Clone)]
pub struct SessionState {
    pub unknown_commands_count: u32,
    pub llm_calls_made: u32,
    pub session_start: Instant,
    pub commands_processed: u32,
    pub last_command_time: Option<Instant>,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            unknown_commands_count: 0,
            llm_calls_made: 0,
            session_start: Instant::now(),
            commands_processed: 0,
            last_command_time: None,
        }
    }

    pub fn session_duration(&self) -> Duration {
        self.session_start.elapsed()
    }

    pub fn time_since_last_command(&self) -> Option<Duration> {
        self.last_command_time.map(|t| t.elapsed())
    }

    pub fn is_likely_bot(&self) -> bool {
        // Bot indicators: rapid commands (< 100ms between), high command rate
        if let Some(time_since_last) = self.time_since_last_command() {
            if time_since_last < Duration::from_millis(100) && self.commands_processed > 3 {
                return true;
            }
        }
        false
    }
}

/// Configuration for LLM escalation behavior
#[derive(Debug, Clone)]
pub struct LlmEscalationConfig {
    pub enabled: bool,
    pub unknown_command_threshold: u32,
    pub max_llm_calls_per_session: u32,
    pub use_llm_for_human_like: bool,
    pub always_escalate_patterns: HashSet<String>,
    pub never_escalate_patterns: HashSet<String>,
}

impl Default for LlmEscalationConfig {
    fn default() -> Self {
        let mut always_escalate = HashSet::new();
        always_escalate.insert("python".to_string());
        always_escalate.insert("ruby".to_string());
        always_escalate.insert("perl".to_string());
        always_escalate.insert("bash".to_string());
        always_escalate.insert("sh -i".to_string());

        let mut never_escalate = HashSet::new();
        never_escalate.insert("nmap".to_string());
        never_escalate.insert("masscan".to_string());
        never_escalate.insert("zgrab".to_string());

        Self {
            enabled: true,
            unknown_command_threshold: 3,
            max_llm_calls_per_session: 10,
            use_llm_for_human_like: true,
            always_escalate_patterns: always_escalate,
            never_escalate_patterns: never_escalate,
        }
    }
}

impl LlmEscalationConfig {
    /// Determine if LLM should be used based on input and session state
    pub fn should_use_llm(
        &self,
        input: &str,
        is_known_command: bool,
        session: &SessionState,
    ) -> bool {
        if !self.enabled {
            return false;
        }

        // Check budget limits
        if session.llm_calls_made >= self.max_llm_calls_per_session {
            return false;
        }

        // Check never escalate patterns
        let input_lower = input.to_lowercase();
        for pattern in &self.never_escalate_patterns {
            if input_lower.contains(pattern) {
                return false;
            }
        }

        // Check always escalate patterns
        for pattern in &self.always_escalate_patterns {
            if input_lower.contains(pattern) {
                return true;
            }
        }

        // If command is known, use native handler
        if is_known_command {
            return false;
        }

        // If we've hit unknown command threshold, escalate
        if session.unknown_commands_count >= self.unknown_command_threshold {
            return true;
        }

        // If bot-like behavior, avoid LLM
        if !self.use_llm_for_human_like && session.is_likely_bot() {
            return false;
        }

        // Default: use native handler
        false
    }
}

/// Base trait for protocol handlers
#[async_trait::async_trait]
pub trait ProtocolHandler {
    /// Handle an incoming connection
    async fn handle_connection<S>(&mut self, stream: S)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_new() {
        let state = SessionState::new();
        assert_eq!(state.unknown_commands_count, 0);
        assert_eq!(state.llm_calls_made, 0);
        assert_eq!(state.commands_processed, 0);
        assert!(state.last_command_time.is_none());
    }

    #[test]
    fn test_session_state_duration() {
        let state = SessionState::new();
        let duration = state.session_duration();
        assert!(duration.as_secs() < 1); // Should be very small
    }

    #[test]
    fn test_session_state_bot_detection_fast_commands() {
        let mut state = SessionState::new();
        state.last_command_time = Some(Instant::now() - Duration::from_millis(50));
        state.commands_processed = 5;
        assert!(state.is_likely_bot());
    }

    #[test]
    fn test_session_state_bot_detection_slow_commands() {
        let mut state = SessionState::new();
        state.last_command_time = Some(Instant::now() - Duration::from_millis(500));
        state.commands_processed = 5;
        assert!(!state.is_likely_bot());
    }

    #[test]
    fn test_session_state_bot_detection_few_commands() {
        let mut state = SessionState::new();
        state.last_command_time = Some(Instant::now() - Duration::from_millis(50));
        state.commands_processed = 2; // Not enough commands to be bot
        assert!(!state.is_likely_bot());
    }

    #[test]
    fn test_llm_config_default() {
        let config = LlmEscalationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.unknown_command_threshold, 3);
        assert_eq!(config.max_llm_calls_per_session, 10);
        assert!(config.use_llm_for_human_like);
        assert!(config.always_escalate_patterns.contains("python"));
        assert!(config.never_escalate_patterns.contains("nmap"));
    }

    #[test]
    fn test_should_use_llm_disabled() {
        let mut config = LlmEscalationConfig::default();
        config.enabled = false;
        let session = SessionState::new();
        assert!(!config.should_use_llm("ls", true, &session));
    }

    #[test]
    fn test_should_use_llm_budget_exceeded() {
        let config = LlmEscalationConfig::default();
        let mut session = SessionState::new();
        session.llm_calls_made = 10; // Max limit
        assert!(!config.should_use_llm("unknown_command", false, &session));
    }

    #[test]
    fn test_should_use_llm_never_escalate_pattern() {
        let config = LlmEscalationConfig::default();
        let session = SessionState::new();
        assert!(!config.should_use_llm("nmap -sV localhost", false, &session));
    }

    #[test]
    fn test_should_use_llm_always_escalate_pattern() {
        let config = LlmEscalationConfig::default();
        let session = SessionState::new();
        assert!(config.should_use_llm("python exploit.py", false, &session));
    }

    #[test]
    fn test_should_use_llm_known_command() {
        let config = LlmEscalationConfig::default();
        let session = SessionState::new();
        assert!(!config.should_use_llm("ls", true, &session));
    }

    #[test]
    fn test_should_use_llm_unknown_threshold_reached() {
        let config = LlmEscalationConfig::default();
        let mut session = SessionState::new();
        session.unknown_commands_count = 3; // Threshold
        assert!(config.should_use_llm("unknown_cmd", false, &session));
    }

    #[test]
    fn test_should_use_llm_unknown_below_threshold() {
        let config = LlmEscalationConfig::default();
        let mut session = SessionState::new();
        session.unknown_commands_count = 2; // Below threshold
        assert!(!config.should_use_llm("unknown_cmd", false, &session));
    }

    #[test]
    fn test_should_use_llm_bot_behavior() {
        let mut config = LlmEscalationConfig::default();
        config.use_llm_for_human_like = true;
        let mut session = SessionState::new();
        session.last_command_time = Some(Instant::now() - Duration::from_millis(50));
        session.commands_processed = 5;
        // Bot-like behavior shouldn't automatically trigger LLM for unknown commands below threshold
        assert!(!config.should_use_llm("unknown_cmd", false, &session));
    }
}
