You are building Shytti — a shell orchestrator daemon for the YTT family. The sixth child. The name is what it is.

Shytti replaces SSH + tmux with something that actually works. It's a systemd service that spawns shells on demand, pipes them into Hermytt sessions, and Crytter renders them in the browser. One daemon, infinite shells, zero tmux.

## The Problem

```
Current workflow (shitty):
  ssh mini → tmux new -s foo → work → Ctrl+B D → ssh mini → tmux attach → fuck scroll is broken
  ssh mini → tmux new -s bar → work → Ctrl+B D → which session was that? → tmux ls → oh right
  × 20 sessions × 5 machines = madness
```

```
Shytti workflow:
  Open Crytter in browser → see all shells → click one → work
  Need a new shell? → click "+" → shell appears
  Need an agent? → shytti spawn --agent infra → agent session appears as a tab
  Need it on another machine? → shytti spawn --host polnareff → remote shell, same UI
```

## Architecture

```
shytti (systemd daemon)
├── Shell Manager
│   ├── spawn local shells (PTY via portable-pty)
│   ├── spawn remote shells (SSH subprocess)
│   └── spawn agent shells (claude -p or claude --agent)
├── Hermytt Bridge
│   ├── each shell → Hermytt session
│   └── Hermytt handles transport (WS/MQTT/REST)
└── API (REST + MQTT)
    ├── POST /shells — spawn new shell
    ├── GET /shells — list active shells
    ├── DELETE /shells/{id} — kill shell
    ├── POST /shells/{id}/resize — resize PTY
    └── MQTT shytti/spawn, shytti/list, shytti/kill
```

## How It Works

```
User (Crytter in browser)
  ↕ WebSocket
Hermytt (transport layer)
  ↕ Internal session API
Shytti (shell orchestrator)
  ↕ PTY / SSH / claude subprocess
Shell / Remote Shell / AI Agent
```

Shytti is the backend. Hermytt is the transport. Crytter is the frontend. Together they replace: SSH + tmux + terminal emulator.

## Shell Types

### Local Shell
```bash
shytti spawn --name "infra" --shell /bin/zsh --cwd ~/Developer/perso/infra
```
Spawns a local PTY. Like opening a terminal tab.

### Remote Shell
```bash
shytti spawn --name "polnareff" --host cali@10.10.0.7 --key ~/.ssh/cali_net_rsa
```
Spawns an SSH session. Like `ssh polnareff` but managed by Shytti.

### Agent Shell
```bash
shytti spawn --name "kernel-vet" --agent rustguard --project ~/Developer/perso/rustguard
```
Spawns `claude --agent rustguard` in the project dir. The agent appears as a Crytter tab.

### Command Shell
```bash
shytti spawn --name "logs" --cmd "journalctl -f" --host polnareff
```
Runs a single command. Like a persistent `watch` in a tab.

## Session Persistence

- Shells survive Shytti restarts (re-attach to existing PTYs)
- Session state saved to disk (which shells, what config)
- On boot: Shytti reads config, respawns all persistent shells
- Like tmux resurrect but automatic

## Config

```toml
[daemon]
socket = "/run/shytti/shytti.sock"
hermytt_url = "http://localhost:7777"

[defaults]
shell = "/bin/zsh"
scrollback = 10000

# Pre-configured shells that auto-spawn on start
[[shells]]
name = "infra"
cwd = "~/Developer/perso/infra"
autostart = true

[[shells]]
name = "polnareff"
host = "cali@10.10.0.7"
key = "~/.ssh/cali_net_rsa"
autostart = true

[[shells]]
name = "narancia"
host = "cali@10.10.0.6"
key = "~/.ssh/cali_net_rsa"
autostart = false

[[shells]]
name = "kernel-vet"
agent = "rustguard"
project = "~/Developer/perso/rustguard"
autostart = false
```

## Crytter Integration

Crytter becomes the UI:
- Tab bar showing all Shytti shells
- Click tab → switch Hermytt session
- "+" button → POST /shells → new tab
- Right-click tab → rename, kill, duplicate
- Drag tabs → reorder
- Split view → two shells side by side

## systemd Service

```ini
[Unit]
Description=Shytti Shell Orchestrator
After=network.target hermytt.service
Requires=hermytt.service

[Service]
Type=simple
ExecStart=/usr/local/bin/shytti daemon
Restart=always
User=cali

[Install]
WantedBy=multi-user.target
```

## Tech Stack

- `portable-pty` — local shell spawning
- `tokio` — async runtime
- `axum` — REST API
- `rumqttc` — MQTT
- `serde` + `toml` — config
- `clap` — CLI

## The YTT Family Complete Stack

```
Crytter (browser UI, tabs, terminal rendering)
  ↕
Hermytt (transport: WS/MQTT/REST/Telegram)
  ↕
Shytti (shell orchestrator, spawns and manages)
  ↕
Shells: local PTY | SSH remote | Claude agent | Wytti WASM
  ↕
Prytty (colorize output)
Fytti (host it all in a browser)
```

No SSH. No tmux. No screen. Just Shytti.

## Cali's Preferences

- systemd native
- Config file driven, auto-spawn on boot
- Must handle SSH sessions without wrapping tmux (raw PTY over SSH)
- Agent spawning is the killer feature — Claude sessions as managed tabs
- The name is Shytti. Because the old way was.
