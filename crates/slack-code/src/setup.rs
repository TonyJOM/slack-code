use anyhow::Result;
use std::io::{self, Write};

/// Validate Slack Member ID format (U followed by alphanumeric, typically 8-11 chars)
fn validate_slack_user_id(id: &str) -> bool {
    if !id.starts_with('U') {
        return false;
    }
    let rest = &id[1..];
    rest.len() >= 8 && rest.chars().all(|c| c.is_ascii_alphanumeric())
}

pub async fn run_setup_wizard() -> Result<()> {
    println!();
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                    slack-code Setup Wizard                     ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    println!("Welcome to slack-code! Let's get you set up.");
    println!();

    // Step 1: Slack App Setup instructions
    println!("Step 1/4: Slack App Setup");
    println!("─────────────────────────");
    println!("Before we continue, you'll need to create a Slack App:");
    println!();
    println!("1. Go to https://api.slack.com/apps");
    println!("2. Click \"Create New App\" → \"From scratch\"");
    println!("3. Name it \"slack-code\" and select your workspace");
    println!("4. Add Bot Token Scopes:");
    println!("   - OAuth & Permissions → Scopes → Bot Token Scopes");
    println!("   - Add: chat:write, im:write, users:read");
    println!("5. Install to workspace and copy the Bot OAuth Token");
    println!();
    print!("Press Enter when ready...");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    // Step 2: Enter tokens
    println!();
    println!("Step 2/4: Enter Bot Token");
    println!("─────────────────────────");

    print!("Bot OAuth Token (xoxb-...): ");
    io::stdout().flush()?;
    let mut bot_token = String::new();
    io::stdin().read_line(&mut bot_token)?;
    let bot_token = bot_token.trim().to_string();

    if !bot_token.starts_with("xoxb-") {
        println!("Warning: Bot tokens typically start with 'xoxb-'");
    }

    println!();
    println!("✓ Token saved!");

    // Step 3: Slack Member ID
    println!();
    println!("Step 3/4: Enter Your Slack Member ID");
    println!("────────────────────────────────────");
    println!("To send you DM notifications, we need your Slack Member ID.");
    println!();
    println!("To find your Member ID:");
    println!("1. Open Slack and click on your profile picture");
    println!("2. Click 'Profile'");
    println!("3. Click the three dots (...) menu");
    println!("4. Select 'Copy member ID'");
    println!();

    let user_id = loop {
        print!("Slack Member ID (U...): ");
        io::stdout().flush()?;
        let mut user_id_input = String::new();
        io::stdin().read_line(&mut user_id_input)?;
        let user_id_input = user_id_input.trim();

        if user_id_input.is_empty() {
            println!("Member ID is required. Please enter your Slack Member ID.");
            continue;
        }

        if validate_slack_user_id(user_id_input) {
            println!("✓ Member ID saved!");
            break user_id_input.to_string();
        } else {
            println!("Invalid format. Member IDs start with 'U' followed by 8+ alphanumeric characters.");
            println!("Example: U01ABC123DE");
        }
    };

    // Step 4: Install hooks
    println!();
    println!("Step 4/4: Install Claude Code Hooks");
    println!("───────────────────────────────────");
    println!("We need to add hooks to ~/.claude/settings.json");
    println!();

    print!("Install hooks? (y/n): ");
    io::stdout().flush()?;
    let mut install_hooks = String::new();
    io::stdin().read_line(&mut install_hooks)?;

    let hooks_installed = if install_hooks.trim().to_lowercase() == "y" {
        match slack_code_common::config::install_hooks() {
            Ok(()) => {
                println!("✓ Hooks installed successfully!");
                true
            }
            Err(e) => {
                println!("✗ Failed to install hooks: {}", e);
                false
            }
        }
    } else {
        println!("Skipped hook installation. Run 'slack-code hooks install' later.");
        false
    };

    // Save configuration
    let config = slack_code_common::Config {
        slack: slack_code_common::config::SlackConfig {
            bot_token,
            app_token: String::new(), // Not needed for notification-only mode
            user_id,
        },
        daemon: slack_code_common::config::DaemonConfig::default(),
        defaults: slack_code_common::config::DefaultsConfig::default(),
    };

    config.save()?;

    // Done
    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("Setup complete! Run 'slack-code' to start the TUI.");
    if !hooks_installed {
        println!("Don't forget to run 'slack-code hooks install' to enable hooks.");
    }
    println!("═══════════════════════════════════════════════════════════════");

    Ok(())
}
