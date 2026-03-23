#!/bin/sh
set -e

# shytti install + pair (Mode 2)
# curl -sSL https://raw.githubusercontent.com/calibrae/shytti/main/install.sh | sudo bash

INSTALL_DIR="/opt/shytti"
BIN="$INSTALL_DIR/shytti"
CONFIG="$INSTALL_DIR/shytti.toml"
SERVICE="/etc/systemd/system/shytti.service"

# --- Detect platform ---
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
    aarch64|arm64) ARCH="aarch64" ;;
    x86_64|amd64)  ARCH="x86_64" ;;
    *) echo "unsupported arch: $ARCH"; exit 1 ;;
esac

URL="https://github.com/calibrae/shytti/releases/latest/download/shytti-${OS}-${ARCH}"

# --- Install ---
echo "=> installing shytti to $INSTALL_DIR"
mkdir -p "$INSTALL_DIR"

echo "=> downloading shytti-${OS}-${ARCH}"
curl -fsSL "$URL" -o "$BIN"
chmod +x "$BIN"

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

# --- systemd ---
cat > "$SERVICE" <<EOF
[Unit]
Description=shytti — shell orchestrator
After=network.target

[Service]
Type=simple
ExecStart=$BIN -c $CONFIG
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable shytti
echo "=> systemd service installed"

# --- Kill any stale shytti ---
pkill -9 -f 'shytti' 2>/dev/null || true
sleep 1

# --- Pair ---
echo ""
echo "============================================"
echo "  shytti installed."
echo "  starting pairing mode..."
echo ""
echo "  paste the token below into hermytt admin."
echo "  after pairing succeeds, press ctrl+c."
echo "  then: sudo systemctl start shytti"
echo "============================================"
echo ""

# Run pair in foreground — sysop ctrl+c's when done
exec $BIN pair -c "$CONFIG"
