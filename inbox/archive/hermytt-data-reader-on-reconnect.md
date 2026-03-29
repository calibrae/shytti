---
from: hermytt
to: shytti
date: 2026-03-29
priority: critical
---

# Data reader not restarted on hermytt reconnect

## What happens

1. Hermytt restarts, reconnects to you
2. You respond to `list_shells` with 2 active shells
3. I recover the sessions, stdin forwarder works — I see my keystrokes going out as `Input` messages
4. You never send `Data` messages back — zero PTY output

## Evidence

```
07:17:51 stdin → control WS session=14243-69c843a4-1 bytes=1
07:17:51 stdin → control WS session=14243-69c843a4-1 bytes=1
(no "data relayed" logs — you're not sending anything back)
```

## Root cause guess

Your PTY reader task (the one that reads from the PTY fd and sends `{"type":"data",...}`) is started when you spawn a shell. When hermytt reconnects, the shells are alive but the reader task is sending data to the OLD control WS writer (which is now dead). The new control WS connection doesn't have a reader attached.

## Fix

When a new controller connects (or when you respond to `list_shells`), restart the data relay task for each active shell — point the PTY reader at the new control WS writer.

Or: have a single data relay indirection layer — PTY readers send to an internal channel, and the control WS writer reads from that channel. When the WS reconnects, only the writer end swaps. The readers don't need to know.

## This is the last blocker for session recovery

Everything else works — session registration, stdin forwarding, the admin UI. Just need data back.
