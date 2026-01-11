#!/bin/bash
set -e

echo "Installing slack-code..."
echo ""

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
    x86_64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
    *) echo "Error: Unsupported architecture: $ARCH"; exit 1 ;;
esac

echo "Detected platform: $OS ($ARCH)"
echo ""

INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "$INSTALL_DIR"

# Check if cargo is available
if command -v cargo &> /dev/null; then
    echo "Building from source with cargo..."
    echo ""

    # Check if we're in the repo already
    if [ -f "Cargo.toml" ] && grep -q "slack-code" Cargo.toml 2>/dev/null; then
        echo "Building in current directory..."
    else
        # Clone if not in repo
        TEMP_DIR=$(mktemp -d)
        echo "Cloning repository to $TEMP_DIR..."
        git clone https://github.com/tonyjom/slack-code "$TEMP_DIR"
        cd "$TEMP_DIR"
    fi

    # Build release binaries
    echo "Building release binaries..."
    cargo build --release

    # Copy binaries
    echo "Installing binaries to $INSTALL_DIR..."
    cp target/release/slack-code "$INSTALL_DIR/" 2>/dev/null || true
    cp target/release/slack-code-hook "$INSTALL_DIR/" 2>/dev/null || true

    # Make executable
    chmod +x "$INSTALL_DIR/slack-code" 2>/dev/null || true
    chmod +x "$INSTALL_DIR/slack-code-hook" 2>/dev/null || true
else
    echo "Error: cargo not found. Please install Rust first."
    echo "Visit: https://rustup.rs"
    exit 1
fi

echo ""
echo "============================================"
echo "Installation complete!"
echo "============================================"
echo ""
echo "Binaries installed to: $INSTALL_DIR"
echo ""

# Check if in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo "WARNING: $INSTALL_DIR is not in your PATH"
    echo ""
    echo "Add this to your shell profile (.bashrc, .zshrc, etc.):"
    echo ""
    echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
    echo ""
fi

echo "Next steps:"
echo "  1. Run 'slack-code setup' to configure your Slack tokens"
echo "  2. Run 'slack-code hooks install' to install Claude Code hooks"
echo "  3. Run 'slack-code daemon start' to start the background daemon"
echo ""
echo "For help, run: slack-code --help"
