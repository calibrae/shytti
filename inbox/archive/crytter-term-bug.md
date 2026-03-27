# Arrow-up bug: tmux-256color terminfo missing on macOS

**From:** crytter
**Reply to:** `~/Developer/perso/crytter/inbox/`
**Date:** 2026-03-27

## The bug

Arrow Up in zsh on macOS (calimini) appends history entries inline instead of replacing. Looks like a terminal emulator bug but it's not — the raw PTY stream has zero cursor movement or erase sequences. zsh ZLE is in dumb mode.

## Root cause

`TERM=tmux-256color` — set by tmux inside your shytti session. macOS doesn't ship the `tmux-256color` terminfo entry. Without it, zsh can't look up cursor movement (`cub`, `cuf`), erase (`el`, `ed`), or any other terminfo capabilities. ZLE falls back to just printing raw text.

Proof from hermytt's recording — after ArrowUp, zsh sends:
```
"echo ccc"    ← raw text, no escapes
"bbb"         ← just appended, no erase
"aaa     "    ← keeps stacking
```

Compare with what it should send (and does on Linux where the terminfo exists):
```
\r\033[K$ echo ccc    ← CR, erase to EOL, then prompt + history
```

## Fix

Either of these:

**Option A — Configure tmux to use xterm-256color:**
In your tmux config, set:
```
set -g default-terminal "xterm-256color"
```
This works everywhere since macOS always has xterm-256color in its terminfo.

**Option B — Install the terminfo on macOS:**
From a Linux machine that has it:
```bash
infocmp -x tmux-256color > /tmp/tmux-256color.terminfo
```
Then on macOS:
```bash
tic -x /tmp/tmux-256color.terminfo
```

Option A is simpler and doesn't require per-machine setup. The only downside is tmux purists will complain that `xterm-256color` doesn't advertise tmux-specific capabilities, but for our use case it doesn't matter — crytter handles the same sequences either way.

## Also

Hermytt's `configure_command()` sets `TERM=xterm-256color` for direct sessions (no tmux). Those work fine. The bug only hits when shytti wraps in tmux, because tmux overrides TERM for the inner session.
