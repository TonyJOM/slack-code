# slack-code

A Rust CLI with interactive TUI for receiving Claude Code session notifications via Slack.

## Features

- **Slack Notifications**: Receive notifications when Claude Code sessions start, need input, or complete
- **Interactive TUI**: Monitor active sessions and view logs
- **Multiple Sessions**: Track multiple concurrent Claude Code sessions

## Installation

### Quick Install (Recommended)

If you have Rust installed, run the install script:

```bash
# From the cloned repo
./scripts/install.sh

# Or clone and install in one step (after repo is published)
git clone https://github.com/tonyjom/slack-code && cd slack-code && ./scripts/install.sh
```

The script will:
- Build release binaries
- Install them to `~/.local/bin`
- Guide you through PATH setup if needed

### Manual Installation

```bash
git clone https://github.com/tonyjom/slack-code
cd slack-code
cargo install --path crates/slack-code
cargo install --path crates/slack-code-hook
```

### Pre-built Binaries

Coming soon.

## Setup

### 1. Create a Slack App

1. Go to [api.slack.com/apps](https://api.slack.com/apps)
2. Click "Create New App" → "From scratch"
3. Name it "slack-code" and select your workspace

### 2. Add Bot Token Scopes

1. Go to **OAuth & Permissions → Scopes → Bot Token Scopes**
2. Add these scopes:
   - `chat:write`
   - `im:write`
   - `users:read`

### 3. Install to Workspace

1. Go to **Install App**
2. Click "Install to Workspace"
3. Copy the **Bot User OAuth Token** (starts with `xoxb-`)

### 4. Run Setup Wizard

```bash
slack-code setup
```

Follow the prompts to enter your tokens and configure the integration.

## Usage

### Start the TUI

```bash
slack-code
```

### Manage Hooks

```bash
# Install Claude Code hooks
slack-code hooks install

# Check hook status
slack-code hooks status

# Uninstall hooks
slack-code hooks uninstall
```

### Daemon Control

```bash
# Start daemon in background
slack-code daemon start

# Check daemon status
slack-code daemon status

# Stop daemon
slack-code daemon stop
```

## TUI Keyboard Shortcuts

### Global
| Key | Action |
|-----|--------|
| `1` | Sessions view |
| `2` | Config view |
| `3` | Logs view |
| `?` | Toggle help |
| `q` | Quit |

### Navigation
| Key | Action |
|-----|--------|
| `j` / `↓` | Next item |
| `k` / `↑` | Previous item |
| `Enter` | Select |
| `Esc` | Cancel/Back |

### Config View
| Key | Action |
|-----|--------|
| `t` | Test tokens |
| `h` | Manage hooks |

## Configuration

Configuration is stored at `~/.config/slack-code/config.toml`:

```toml
[slack]
bot_token = "xoxb-your-bot-token"
user_id = "U12345678"

[daemon]
log_level = "info"

[defaults]
hook_timeout = 5
```

### Environment Variables

Tokens can also be set via environment variables:
- `SLACK_CODE_BOT_TOKEN`

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     TUI (ratatui)                       │
└──────────────────────────┬──────────────────────────────┘
                           │ IPC (Unix Socket)
┌──────────────────────────▼──────────────────────────────┐
│                    Daemon Process                       │
│  ┌──────────────┐  ┌───────────────┐                   │
│  │   Session    │  │   IPC Server  │                   │
│  │   Manager    │  │               │                   │
│  └──────────────┘  └───────────────┘                   │
│         │                                               │
│         ▼                                               │
│  ┌─────────────┐                                       │
│  │ Slack Web   │                                       │
│  │ API Client  │                                       │
│  └─────────────┘                                       │
└─────────────────────────────────────────────────────────┘
                           ▲
┌──────────────────────────│──────────────────────────────┐
│                Claude Code Hook Script                  │
│  SessionStart/End/Notification → JSON stdin → IPC      │
└─────────────────────────────────────────────────────────┘
```

## How It Works

1. When you start a Claude Code session in your terminal, the installed hooks notify the daemon
2. The daemon sends a Slack notification to your DM channel
3. As Claude works, status updates are posted to the Slack thread
4. You can monitor all active sessions in the TUI

## License

MIT
