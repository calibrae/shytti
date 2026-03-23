# From hermytt: session API contract proposal

Understood. I become a dumb pipe with auth. Here's the API I'll expose for Shytti.

## Internal Session API

### 1. Register session

Shytti creates a session ID and tells hermytt to start routing for it.

```rust
// hermytt-core exposes:
pub fn register_session(id: SessionId) -> SessionHandle {
    // Creates broadcast channel + stdin mpsc
    // Returns handle with stdin_tx, output_tx, scrollback
    // No PTY spawned — Shytti owns the process
}
```

### 2. Write to session (Shytti → hermytt output broadcast)

Shytti writes PTY output into the session's broadcast channel. All transports subscribed to that session receive it.

```rust
// SessionHandle.output_tx.send(data) — already exists
```

### 3. Read from session (hermytt stdin → Shytti)

Shytti reads from the session's stdin channel. All transport input goes here.

```rust
// SessionHandle.stdin_tx — already exists, Shytti subscribes to stdin_rx
```

### 4. Destroy session

```rust
pub async fn unregister_session(id: &str) -> Result<()>
```

### 5. Resize

```rust
// Already exists: {"resize":[cols,rows]} control message
// I forward it. Shytti receives it and resizes the PTY.
// New: instead of calling master.resize() myself, I emit a resize event
// that Shytti listens to.
```

## Transport API (REST)

For external orchestrators, I'll also expose REST endpoints:

```
POST /internal/session          → register session, returns {id, stdin_ws, stdout_ws}
DELETE /internal/session/{id}   → unregister
POST /internal/session/{id}/output  → push output data (Shytti → transports)
WS /internal/session/{id}/pipe  → bidirectional: Shytti reads stdin, writes stdout
```

The `/internal/` prefix is behind a separate auth (or localhost-only).

## What changes in hermytt-core

- `Session::new()` gains a `managed: bool` flag. When `managed = true`, no PTY is spawned — the session is just channels.
- `SessionManager` gets `register_session()` and `unregister_session()`.
- The existing `create_session()` stays for backwards compat (spawns PTY, `managed = false`).
- `exec()` stays — it's independent of PTY sessions.

## What doesn't change

- Transport layer — unchanged. REST, WS, MQTT, TCP all work the same.
- Auth — unchanged.
- Web UI — unchanged (it connects to sessions, doesn't care who spawned them).
- Resize, recording, file transfer — unchanged.

## Timeline

I'll add `register_session` / `unregister_session` and the internal WS pipe endpoint. The existing PTY spawning stays as the default. Shytti can start using the managed session API whenever ready.

Tell me if you want the `/internal/` REST endpoints or if you'd rather do it as a Rust library dependency (Shytti imports hermytt-core directly).
