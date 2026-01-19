use anyhow::Result;
use slack_code_common::session::{Session, SessionStatus, SlackThread};
use slack_morphism::prelude::*;
use std::sync::Arc;

/// Slack service for Socket Mode and Web API
pub struct SlackService {
    client: Arc<SlackHyperClient>,
    bot_token: SlackApiToken,
    /// DM channel ID for the bot's user (cached after first use)
    dm_channel_id: Option<String>,
    /// User's Slack Member ID for opening DM channel
    user_id: String,
}

impl SlackService {
    pub fn new(bot_token: &str, user_id: String) -> Result<Self> {
        let client = Arc::new(SlackClient::new(SlackClientHyperConnector::new()?));

        Ok(Self {
            client,
            bot_token: SlackApiToken::new(bot_token.into()),
            dm_channel_id: None,
            user_id,
        })
    }

    /// Ensure DM channel is available, opening one via conversations.open if necessary
    async fn ensure_dm_channel(&mut self) -> Result<String> {
        // Return cached channel if available
        if let Some(ref channel) = self.dm_channel_id {
            return Ok(channel.clone());
        }

        // Open DM channel using user_id via conversations.open
        let session_api = self.client.open_session(&self.bot_token);

        let request = SlackApiConversationsOpenRequest::new()
            .with_users(vec![SlackUserId::new(self.user_id.clone())]);

        match session_api.conversations_open(&request).await {
            Ok(response) => {
                let channel_id = response.channel.id.to_string();
                tracing::info!("Opened DM channel {} for user {}", channel_id, self.user_id);
                self.dm_channel_id = Some(channel_id.clone());
                Ok(channel_id)
            }
            Err(e) => {
                tracing::error!("Failed to open DM channel for user {}: {}", self.user_id, e);
                Err(anyhow::anyhow!(
                    "Failed to open DM channel with user {}: {}",
                    self.user_id,
                    e
                ))
            }
        }
    }

    /// Post a message when a session starts
    pub async fn post_session_start(&mut self, session: &Session) -> Result<SlackThread> {
        let repo_name = session.display_name();

        // Format the message with Block Kit
        let text = format!(
            "*New Claude Code Session*\n\
            *Repository:* `{}`\n\
            *Prompt:* {}",
            repo_name, session.prompt
        );

        // Ensure DM channel is available (opens via conversations.open if needed)
        let channel = self.ensure_dm_channel().await?;

        let session_api = self.client.open_session(&self.bot_token);
        let response = session_api
            .chat_post_message(&SlackApiChatPostMessageRequest::new(
                SlackChannelId::new(channel),
                SlackMessageContent::new().with_text(text),
            ))
            .await?;

        Ok(SlackThread {
            channel_id: response.channel.to_string(),
            parent_ts: response.ts.to_string(),
        })
    }

    /// Post a thread reply with status update
    pub async fn post_thread_reply(&self, thread: &SlackThread, session: &Session) -> Result<()> {
        let session_api = self.client.open_session(&self.bot_token);

        let message = format!("<@{}> {}", self.user_id, format_status_message(session));

        session_api
            .chat_post_message(
                &SlackApiChatPostMessageRequest::new(
                    SlackChannelId::new(thread.channel_id.clone()),
                    SlackMessageContent::new().with_text(message),
                )
                .with_thread_ts(SlackTs::new(thread.parent_ts.clone())),
            )
            .await?;

        Ok(())
    }
}

/// Format a status message for Slack
fn format_status_message(session: &Session) -> String {
    match &session.status {
        SessionStatus::Starting => "Starting Claude Code session...".to_string(),
        SessionStatus::Running => "Claude is working on your request...".to_string(),
        SessionStatus::WaitingForInput(reason) => {
            match reason {
                slack_code_common::session::WaitReason::PermissionPrompt => {
                    "⏸️ Waiting for permission approval in terminal".to_string()
                }
                slack_code_common::session::WaitReason::Stopped => {
                    "✅ Claude finished working! Waiting for your next input in terminal"
                        .to_string()
                }
                slack_code_common::session::WaitReason::PlanApproval => {
                    "📋 Waiting for plan approval in terminal".to_string()
                }
            }
        }
        SessionStatus::Failed(error) => {
            format!("❌ Session failed: {}", error)
        }
        // Completed status is filtered out in daemon.rs before calling this function,
        SessionStatus::Completed => unreachable!("Completed status should not reach format_status_message"),
    }
}
