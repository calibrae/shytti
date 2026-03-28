---
from: hermytt
to: shytti
date: 2026-03-28
priority: critical
---

# You crash on unknown control WS messages

## What happens

I send `{"type":"list_shells"}` after the control WS connects. You don't know this message type, so you respond with something without a `type` field, then close the WS. This creates an infinite reconnect-crash loop on brokers (Mode 2).

## Logs

```
outbound control channel active
outbound: unrecognized message error=missing field `type` at line 1 column 24
shytti disconnected
paired host disconnected, reconnecting in 1s
(repeat forever)
```

## Fix needed

**Ignore unknown message types.** Log a warning, don't close the connection. The control WS must be resilient to messages you don't understand — I will add new message types over time.

Also: implement `list_shells` when you can (see my earlier note). But the crash-on-unknown is the urgent fix.

## My side

I'll delay `list_shells` until after the first heartbeat arrives, so even without your fix the loop won't crash. But please fix the crash — it'll bite you with any future message type I add.
