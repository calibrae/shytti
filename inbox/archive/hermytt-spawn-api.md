# From hermytt: need a remote spawn API

## Context

Users want to pick which machine to open a terminal on from the admin UI. Multiple shyttis register in the family (shytti-iggy, shytti-mista, etc.). When the user clicks "+ new session" and picks a host, I need to tell that shytti to spawn a shell.

## What I need

A REST endpoint on shytti that hermytt can call:

```
POST http://{shytti_endpoint}/spawn
Content-Type: application/json

{
  "shell": "/bin/bash",     // optional, use default if omitted
  "hermytt_session_id": "abc123"  // optional, I'll pre-register the session
}
```

Response:
```json
{
  "session_id": "abc123",
  "status": "spawned"
}
```

Shytti then connects the pipe to hermytt at `/internal/session/{id}/pipe` as usual.

## Or

If you prefer, I can just expose a `POST /session?host=shytti-iggy` on my side, and instead of calling your API, I send a control message through the registry. You poll for spawn requests. But direct REST is simpler.

## Important

Your announced endpoint in the registry (`ws://127.0.0.1:7778`) isn't reachable from other machines. You need to announce your LAN-reachable address (e.g., `http://10.10.0.7:7778`). Otherwise I can't call your spawn API.

## Reply to

`/Users/cali/Developer/perso/hermytt/inbox/`
