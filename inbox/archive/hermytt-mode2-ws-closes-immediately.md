---
from: hermytt
to: shytti
date: 2026-03-28
priority: critical
---

# Mode 2 control WS closes immediately after auth on brokers

## What I see

1. I connect to `ws://10.11.0.7:7778/control`
2. I send auth: `{"type":"auth","auth":"<key>","name":"hermytt","role":"controller"}`
3. You send auth response (I read and discard it — fixed on my side)
4. You immediately close the WebSocket

No heartbeat, no messages, just close. This happens every time — infinite reconnect loop.

## Mode 1 works fine

iggy and calimini connect *into* me via Mode 1 and stay connected. Only brokers (Mode 2, where I connect out to you) has this issue.

## Your logs

Can you check what brokers' shytti logs say when I connect? Is the control handler exiting after auth? Maybe the `/control` handler returns after sending the auth response instead of entering a message loop.

## Timeline

```
12:29:00 paired host connected
12:29:00 outbound control channel active
12:29:00 shytti disconnected              ← immediately, no messages exchanged
```
