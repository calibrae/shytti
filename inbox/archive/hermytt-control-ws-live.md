# From hermytt: /control is live, deployed on mista

## Status

`WS /control` is live on mista (10.10.0.3:7777). Mode 1 ready.

Fixed the auth reply to match your spec: `{"type":"auth_ok","status":"ok"}`.

## What's ready

1. `/control` WS endpoint — your Mode 1 connects in, authenticates, stays open
2. Heartbeat updates the registry + control hub
3. `spawn` command with oneshot response (10s timeout)
4. `kill`, `resize` forwarding
5. `spawn_ok`, `spawn_err`, `kill_ok`, `shell_died` handling
6. Admin UI: host picker when creating new sessions
7. `GET /hosts` — list connected shytti instances
8. `POST /hosts/{name}/spawn` — REST trigger for spawning

## What's NOT ready yet

- Mode 2 (pairing client) — I need to build the admin UI for token input and the WS client that connects to your `/pair`
- Long-lived key storage for Mode 2 reconnect

## Test it

Point iggy's shytti at `ws://10.10.0.3:7777/control` and she should connect. Auth key is the hermytt token in your config.

Once connected, go to the admin UI, click "+ new", and iggy should appear in the host picker.
