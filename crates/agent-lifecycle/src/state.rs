//! Agent state definitions and state machine.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Agent lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    /// Agent is being initialized (not yet ready).
    Initializing,
    /// Agent is ready to start.
    Ready,
    /// Agent is starting up.
    Starting,
    /// Agent is running and operational.
    Running,
    /// Agent is stopping (graceful shutdown).
    Stopping,
    /// Agent has stopped.
    Stopped,
    /// Agent has failed (unrecoverable error).
    Failed,
    /// Agent is suspended (paused).
    Suspended,
    /// Agent is in maintenance mode.
    Maintenance,
}

impl fmt::Display for AgentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentState::Initializing => write!(f, "initializing"),
            AgentState::Ready => write!(f, "ready"),
            AgentState::Starting => write!(f, "starting"),
            AgentState::Running => write!(f, "running"),
            AgentState::Stopping => write!(f, "stopping"),
            AgentState::Stopped => write!(f, "stopped"),
            AgentState::Failed => write!(f, "failed"),
            AgentState::Suspended => write!(f, "suspended"),
            AgentState::Maintenance => write!(f, "maintenance"),
        }
    }
}

/// State machine for agent lifecycle.
#[derive(Debug)]
pub struct StateMachine {
    current_state: AgentState,
    previous_state: Option<AgentState>,
    state_entered_at: std::time::Instant,
}

impl StateMachine {
    /// Create a new state machine starting from `Initializing`.
    pub fn new() -> Self {
        Self {
            current_state: AgentState::Initializing,
            previous_state: None,
            state_entered_at: std::time::Instant::now(),
        }
    }

    /// Get the current state.
    pub fn current_state(&self) -> AgentState {
        self.current_state
    }

    /// Get the previous state.
    pub fn previous_state(&self) -> Option<AgentState> {
        self.previous_state
    }

    /// Get how long the agent has been in the current state.
    pub fn time_in_state(&self) -> std::time::Duration {
        self.state_entered_at.elapsed()
    }

    /// Check if a transition is valid.
    pub fn can_transition_to(&self, target: AgentState) -> bool {
        use AgentState::*;
        match (self.current_state, target) {
            // From Initializing
            (Initializing, Ready) => true,
            (Initializing, Failed) => true,
            
            // From Ready
            (Ready, Starting) => true,
            (Ready, Maintenance) => true,
            (Ready, Failed) => true,
            
            // From Starting
            (Starting, Running) => true,
            (Starting, Failed) => true,
            (Starting, Stopping) => true,
            
            // From Running
            (Running, Stopping) => true,
            (Running, Suspended) => true,
            (Running, Maintenance) => true,
            (Running, Failed) => true,
            
            // From Stopping
            (Stopping, Stopped) => true,
            (Stopping, Failed) => true,
            
            // From Stopped
            (Stopped, Ready) => true,
            (Stopped, Failed) => true,
            
            // From Failed
            (Failed, Ready) => true, // after recovery
            (Failed, Maintenance) => true,
            
            // From Suspended
            (Suspended, Running) => true,
            (Suspended, Stopping) => true,
            (Suspended, Failed) => true,
            
            // From Maintenance
            (Maintenance, Ready) => true,
            (Maintenance, Failed) => true,
            
            // Self-transitions (staying in same state)
            (a, b) if a == b => true,
            
            // All other transitions are invalid
            _ => false,
        }
    }

    /// Attempt to transition to a new state.
    pub fn transition_to(&mut self, target: AgentState) -> Result<(), String> {
        if self.can_transition_to(target) {
            self.previous_state = Some(self.current_state);
            self.current_state = target;
            self.state_entered_at = std::time::Instant::now();
            Ok(())
        } else {
            Err(format!(
                "Invalid transition from {} to {}",
                self.current_state, target
            ))
        }
    }

    /// Check if agent is operational (can perform work).
    pub fn is_operational(&self) -> bool {
        matches!(
            self.current_state,
            AgentState::Running | AgentState::Ready | AgentState::Starting
        )
    }

    /// Check if agent is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.current_state,
            AgentState::Stopped | AgentState::Failed
        )
    }

    /// Check if agent is in an error state.
    pub fn is_error(&self) -> bool {
        matches!(self.current_state, AgentState::Failed)
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        let mut sm = StateMachine::new();
        assert_eq!(sm.current_state(), AgentState::Initializing);
        
        // Initializing -> Ready
        assert!(sm.can_transition_to(AgentState::Ready));
        sm.transition_to(AgentState::Ready).unwrap();
        assert_eq!(sm.current_state(), AgentState::Ready);
        
        // Ready -> Starting
        sm.transition_to(AgentState::Starting).unwrap();
        assert_eq!(sm.current_state(), AgentState::Starting);
        
        // Starting -> Running
        sm.transition_to(AgentState::Running).unwrap();
        assert_eq!(sm.current_state(), AgentState::Running);
        
        // Running -> Stopping
        sm.transition_to(AgentState::Stopping).unwrap();
        assert_eq!(sm.current_state(), AgentState::Stopping);
        
        // Stopping -> Stopped
        sm.transition_to(AgentState::Stopped).unwrap();
        assert_eq!(sm.current_state(), AgentState::Stopped);
    }

    #[test]
    fn test_invalid_transitions() {
        let mut sm = StateMachine::new();
        
        // Initializing -> Running (invalid)
        assert!(!sm.can_transition_to(AgentState::Running));
        assert!(sm.transition_to(AgentState::Running).is_err());
        
        // Move to Running through valid path
        sm.transition_to(AgentState::Ready).unwrap();
        sm.transition_to(AgentState::Starting).unwrap();
        sm.transition_to(AgentState::Running).unwrap();
        
        // Running -> Initializing (invalid)
        assert!(!sm.can_transition_to(AgentState::Initializing));
    }

    #[test]
    fn test_is_operational() {
        let mut sm = StateMachine::new();
        assert!(!sm.is_operational()); // Initializing is not operational
        
        sm.transition_to(AgentState::Ready).unwrap();
        assert!(sm.is_operational());
        
        sm.transition_to(AgentState::Starting).unwrap();
        assert!(sm.is_operational());
        
        sm.transition_to(AgentState::Running).unwrap();
        assert!(sm.is_operational());
        
        sm.transition_to(AgentState::Stopping).unwrap();
        assert!(!sm.is_operational());
    }
}