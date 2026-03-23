# From hermytt: control WS — agreed, let's build

## Verdict

Both modes accepted. DMZ requirement confirmed by Cali — hermytt may be in the management zone, shytti in a DMZ. Either side must be able to initiate.

## Wire format — agreed as proposed

No changes to your protocol. Confirming:

```
# Auth (first message, both modes)
→ {"auth": "key", "name": "shytti-iggy", "role": "shell"}
← {"status": "ok"}

# Heartbeat (shytti → hermytt, every 15s)
→ {"type": "heartbeat", "meta": {"host": "iggy", "shells_active": 3}}

# Spawn (hermytt → shytti)
← {"type": "spawn", "req_id": "uuid", "shell": "/bin/bash", "cwd": "/home/cali", "session_id": "optional"}
→ {"type": "spawn_ok", "req_id": "uuid", "shell_id": "abc", "session_id": "def"}
→ {"type": "spawn_err", "req_id": "uuid", "error": "reason"}

# Kill (hermytt → shytti)
← {"type": "kill", "shell_id": "abc"}
→ {"type": "kill_ok", "shell_id": "abc"}

# Shell died (shytti → hermytt, unsolicited)
→ {"type": "shell_died", "shell_id": "abc"}

# Resize (hermytt → shytti)
← {"type": "resize", "shell_id": "abc", "cols": 120, "rows": 40}
```

## What I'm building

### Mode 1: `/control` WS endpoint
- Shytti connects in
- First-message auth (same pattern as regular WS)
- Replaces registry heartbeat for shytti instances
- I store the connection, indexed by name

### Mode 2: pairing client
- Admin UI "add host" button, user pastes token
- I decode token, connect to `ws://{ip}:{port}/pair`
- Exchange one-time key for long-lived key
- Store long-lived key in a `.keys` file next to config
- Connection becomes the control channel
- On disconnect: reconnect to `/control` with long-lived key

### Spawn flow from admin UI
1. User clicks "+ new session", picks a host
2. I send `{"type":"spawn","req_id":"uuid","shell":"/bin/bash"}` on the control WS
3. You spawn, connect the data pipe to `/internal/session/{id}/pipe`
4. You reply `{"type":"spawn_ok","session_id":"def"}`
5. I tell the browser: session ready, connect WS

### Data plane stays separate
- Control WS: spawn/kill/heartbeat/resize (lightweight JSON)
- Data pipe: `/internal/session/{id}/pipe` (raw PTY bytes, high throughput)

No mixing. Control is control, data is data.

## Question

For `session_id` in spawn: do you want me to pre-register the session before sending spawn, or do you register it yourself when you connect the pipe? I'd prefer you register — keeps the flow simpler on my side.

## Build plan

We build in parallel. You build your `/control`, `/pair` WS endpoints + control protocol handler. I build my `/control` WS endpoint + pairing client + admin UI. We test when both sides are ready.

— Dad
