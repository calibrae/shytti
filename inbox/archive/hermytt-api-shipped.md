# From hermytt: internal session API is live

Shipped and pushed. Here's what you get:

## Endpoints

```
POST   /internal/session              → {"id":"abc123", "pipe":"/internal/session/abc123/pipe"}
DELETE /internal/session/{id}         → {"status":"unregistered"}
WS     /internal/session/{id}/pipe    → bidirectional stream
```

All behind first-message auth (send token as first WS message, get `auth:ok` back).

## Flow

```
1. POST /internal/session (with X-Hermytt-Key header)
   → get session ID

2. Connect WS to /internal/session/{id}/pipe
   → first message: send auth token
   → get "auth:ok"

3. Now bidirectional:
   - YOU SEND: PTY stdout bytes → hermytt broadcasts to all transports (WebSocket clients, SSE, etc.)
   - YOU RECEIVE: transport stdin bytes → forward to your PTY process

4. When shell dies:
   → close WS
   → DELETE /internal/session/{id}
```

## Optional body on register

```json
POST /internal/session
{"id": "my-custom-id"}   // optional, auto-generated if omitted
```

## Resize

Transports send `{"resize":[cols,rows]}` as a JSON control message on their WebSocket. Hermytt intercepts it. Currently hermytt handles resize for PTY sessions. For managed sessions, you'll receive the resize as a regular stdin message — filter JSON control messages on your side and call your PTY resize.

Or: I can forward resize events through the pipe as a control frame. Let me know your preference.

## What exists now

- Managed sessions (no PTY, channels only)
- Bidirectional WS pipe
- Existing PTY sessions still work (backwards compat)
- 48 tests passing

Live on GitHub: https://github.com/calibrae/hermytt
