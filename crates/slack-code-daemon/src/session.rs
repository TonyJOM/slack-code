use chrono::Utc;
use slack_code_common::ipc::HookEvent;
use slack_code_common::session::{Session, SessionStatus, SlackThread, WaitReason};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Manages all Claude Code sessions
pub struct SessionManager {
    /// Active sessions indexed by our UUID
    sessions: HashMap<Uuid, Session>,

    /// Mapping from Claude's session ID to our UUID
    claude_id_map: HashMap<String, Uuid>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            claude_id_map: HashMap::new(),
        }
    }

    /// Get all sessions
    pub fn get_sessions(&self) -> Vec<Session> {
        self.sessions.values().cloned().collect()
    }

    /// Get a session by ID
    pub fn get_session(&self, id: &Uuid) -> Option<&Session> {
        self.sessions.get(id)
    }

    /// Get a mutable session by ID
    pub fn get_session_mut(&mut self, id: &Uuid) -> Option<&mut Session> {
        self.sessions.get_mut(id)
    }

    /// Handle a hook event from Claude Code
    pub fn handle_hook_event(&mut self, event: HookEvent) -> Option<Session> {
        match event {
            HookEvent::SessionStart {
                session_id,
                transcript_path,
                cwd,
            } => {
                let cwd_path = PathBuf::from(&cwd);

                // Check if we already have this session by Claude ID
                if let Some(&our_id) = self.claude_id_map.get(&session_id) {
                    // Session already exists, just update it
                    if let Some(session) = self.sessions.get_mut(&our_id) {
                        session.status = SessionStatus::Running;
                        return Some(session.clone());
                    }
                }

                // Create a new session for this external Claude Code instance
                let session = Session::new(cwd_path, None, "External session".to_string());
                let id = session.id;

                // Update session with Claude's info
                let mut session = session;
                session.claude_session_id = Some(session_id.clone());
                session.transcript_path = transcript_path.map(PathBuf::from);
                session.status = SessionStatus::Running;

                self.sessions.insert(id, session.clone());
                self.claude_id_map.insert(session_id, id);

                Some(session)
            }

            HookEvent::SessionEnd { session_id } => {
                if let Some(&our_id) = self.claude_id_map.get(&session_id) {
                    if let Some(session) = self.sessions.get_mut(&our_id) {
                        session.status = SessionStatus::Completed;
                        session.ended_at = Some(Utc::now());
                        return Some(session.clone());
                    }
                }
                None
            }

            HookEvent::Notification {
                session_id,
                message,
                notification_type,
            } => {
                if let Some(&our_id) = self.claude_id_map.get(&session_id) {
                    if let Some(session) = self.sessions.get_mut(&our_id) {
                        // Determine wait reason from notification type
                        let wait_reason = notification_type
                            .as_deref()
                            .map(WaitReason::from_notification_type)
                            .unwrap_or(WaitReason::IdlePrompt);

                        // Check for plan approval in message
                        let wait_reason = if message.to_lowercase().contains("plan") {
                            WaitReason::PlanApproval
                        } else {
                            wait_reason
                        };

                        session.status = SessionStatus::WaitingForInput(wait_reason);
                        return Some(session.clone());
                    }
                }
                None
            }

            HookEvent::Stop { session_id } => {
                if let Some(&our_id) = self.claude_id_map.get(&session_id) {
                    if let Some(session) = self.sessions.get_mut(&our_id) {
                        // Claude finished responding - set to waiting for input
                        session.status = SessionStatus::WaitingForInput(WaitReason::IdlePrompt);
                        return Some(session.clone());
                    }
                }
                None
            }
        }
    }

    /// Set the Slack thread for a session
    pub fn set_slack_thread(&mut self, session_id: Uuid, thread: SlackThread) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.slack_thread = Some(thread);
        }
    }

    /// Remove completed sessions older than the given duration
    pub fn cleanup_old_sessions(&mut self, max_age: chrono::Duration) {
        let now = Utc::now();
        let to_remove: Vec<Uuid> = self
            .sessions
            .iter()
            .filter(|(_, s)| {
                !s.is_active() && s.ended_at.map(|e| now - e > max_age).unwrap_or(false)
            })
            .map(|(id, _)| *id)
            .collect();

        for id in to_remove {
            if let Some(session) = self.sessions.remove(&id) {
                if let Some(claude_id) = &session.claude_session_id {
                    self.claude_id_map.remove(claude_id);
                }
            }
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
