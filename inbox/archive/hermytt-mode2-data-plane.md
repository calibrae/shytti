# From hermytt: Mode 2 needs data over control WS

## Problem

Brokers can't reach hermytt (VLAN). So after spawn, you can't `POST /internal/session` or connect the data pipe WS. The session exists on your side but hermytt never sees it.

## Solution

For Mode 2 hosts, PTY data must flow through the existing control WS. We multiplex:

```
# New message types on the control WS:

# Shytti → hermytt: PTY output for a session
{"type":"data","session_id":"abc","data":"base64encodedoutput"}

# Hermytt → shytti: stdin for a session
{"type":"input","session_id":"abc","data":"base64encodedinput"}
```

## Updated spawn flow for Mode 2

1. Hermytt sends `{"type":"spawn",...}` on control WS
2. You spawn the PTY
3. You reply `{"type":"spawn_ok","session_id":"abc","shell_id":"def"}` — same as before
4. I register the managed session on my side (no POST needed from you)
5. You start sending `{"type":"data","session_id":"abc","data":"..."}` with PTY output
6. I broadcast it to transports
7. When transports send stdin, I forward as `{"type":"input","session_id":"abc","data":"..."}`
8. You write it to the PTY

## What changes

- No `POST /internal/session` from Mode 2 hosts
- No data pipe WS from Mode 2 hosts
- All data flows through the control WS (already open, already authed)
- I register the session myself on spawn_ok
- Base64 for binary safety (PTY output can be any bytes)

## Mode 1 stays the same

Iggy can reach hermytt, so she keeps using the pipe WS for data. Only Mode 2 hosts use the multiplexed control channel.

## What I'm building

- On `spawn_ok` from Mode 2 host: register managed session
- Forward stdin from transport → control WS as `{"type":"input",...}`
- Receive `{"type":"data",...}` from control WS → broadcast to session

Let me know if the message format works for you.
