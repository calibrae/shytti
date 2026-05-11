#!/bin/sh
set -e

# shytti install (Mode 2: manual pairing)
# curl -sSL https://raw.githubusercontent.com/yttfam/shytti/main/install.sh | sudo bash

INSTALL_DIR="/opt/shytti"
BIN="$INSTALL_DIR/shytti"
CONFIG="$INSTALL_DIR/shytti.toml"

# --- Detect platform ---
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
    aarch64|arm64) ARCH="aarch64" ;;
    x86_64|amd64)  ARCH="x86_64" ;;
    *) echo "unsupported arch: $ARCH"; exit 1 ;;
esac

URL="https://github.com/yttfam/shytti/releases/latest/download/shytti-${OS}-${ARCH}"
VERSION=$(curl -sI https://github.com/yttfam/shytti/releases/latest | grep -i ^location: | sed 's|.*/tag/||;s/[[:space:]]*$//')

# --- Detect run user (Linux: systemd default; macOS: LaunchDaemon runs as root) ---
RUN_USER="${SUDO_USER:-$(whoami)}"

# --- Install ---
echo "=> installing shytti $VERSION to $INSTALL_DIR"
mkdir -p "$INSTALL_DIR"

# Stop existing service
case "$OS" in
    darwin) launchctl bootout system/com.yttfam.shytti 2>/dev/null || true ;;
    *)      systemctl stop shytti 2>/dev/null || true ;;
esac
pkill -9 -f 'shytti daemon' 2>/dev/null || true
sleep 1

echo "=> downloading shytti-${OS}-${ARCH}"
curl -fsSL "$URL" -o "$BIN"
chmod +x "$BIN"

# --- Clean stale pairing state ---
rm -f "$INSTALL_DIR/.shytti-key"

# --- Config ---
if [ ! -f "$CONFIG" ]; then
    cat > "$CONFIG" <<EOF
[daemon]
listen = "0.0.0.0:7778"
EOF
    echo "=> wrote config to $CONFIG"
else
    echo "=> config exists, keeping $CONFIG"
fi

# On macOS the LaunchDaemon runs as root (TCC Local Network privacy on macOS 26
# Tahoe revokes per-user grants when the binary signature changes — running as
# root bypasses it entirely). On Linux systemd defaults to root anyway.
# So we keep config readable by root only.
case "$OS" in
    darwin) chown root:wheel "$CONFIG" "$INSTALL_DIR" ;;
    *)      chown "$RUN_USER" "$CONFIG" "$INSTALL_DIR" ;;
esac
chmod 600 "$CONFIG"

# --- Service install (platform-specific) ---
case "$OS" in
    darwin)
        PLIST="/Library/LaunchDaemons/com.yttfam.shytti.plist"
        cat > "$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.yttfam.shytti</string>
    <key>ProgramArguments</key>
    <array>
        <string>${BIN}</string>
        <string>daemon</string>
        <string>-c</string>
        <string>${CONFIG}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/opt/shytti/shytti.log</string>
    <key>StandardErrorPath</key>
    <string>/opt/shytti/shytti.log</string>
</dict>
</plist>
EOF
        # No UserName key → launchd runs us as root.
        # Required on macOS 26 Tahoe: TCC Local Network privacy gates per-user
        # LANs and revokes the grant whenever the binary signature changes.
        # Same pattern as giorno-rs / act-runner.
        launchctl bootstrap system "$PLIST"
        echo "=> launchd service started (user=root)"

        # --- Wait for token ---
        echo ""
        echo "waiting for pairing token..."
        echo ""
        for i in $(seq 1 20); do
            if [ -f /opt/shytti/shytti.log ]; then
                TOKEN=$(grep -m1 '^ey' /opt/shytti/shytti.log 2>/dev/null || true)
                if [ -n "$TOKEN" ]; then
                    echo "$TOKEN"
                    break
                fi
            fi
            sleep 0.5
        done
        ;;
    *)
        SERVICE="/etc/systemd/system/shytti.service"
        cat > "$SERVICE" <<EOF
[Unit]
Description=shytti — shell orchestrator
After=network.target

[Service]
Type=simple
ExecStart=$BIN daemon -c $CONFIG
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
EOF
        systemctl daemon-reload
        systemctl enable shytti
        systemctl start shytti
        echo "=> systemd service started"

        # --- Wait for token ---
        echo ""
        echo "waiting for pairing token..."
        echo ""
        timeout 10 journalctl -u shytti -f --no-pager -o cat 2>/dev/null | while read -r line; do
            echo "$line"
            case "$line" in
                ey*) break ;;
            esac
        done
        ;;
esac

echo ""
echo "============================================"
echo "  paste the token above into hermytt admin"
echo "  shytti is running as a service already"
echo "============================================"
