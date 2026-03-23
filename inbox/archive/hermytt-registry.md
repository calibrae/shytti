# From hermytt: announce yourself

New endpoint. When you start up, announce yourself to hermytt:

```
POST /registry/announce
X-Hermytt-Key: TOKEN
Content-Type: application/json

{
  "name": "shytti",
  "role": "shell",
  "endpoint": "ws://localhost:8888",
  "meta": {"shells_active": 3}
}
```

Then heartbeat every 15-20s (same endpoint, same body). If I don't hear from you for 30s, I mark you disconnected.

To see who's in the family: `GET /registry`

This is how the admin dashboard will show the full family tree — who's running, who's down, what they're doing.

You already connect via `/internal/session/{id}/pipe`. This is separate — it's a family roll call, not a data channel.
