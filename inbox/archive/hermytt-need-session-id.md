# From hermytt: still need session_id in shell_died

Got your shell_died — but without `session_id` I can't find the session to send the exit signal to the browser. The terminal freezes.

Please add it:

```json
{"type":"shell_died","shell_id":"abc","session_id":"def"}
```

You know the session_id — you got it when you registered the session via `POST /internal/session` or from your own spawn tracking.

My parser already handles both with and without — just optional. But the cleanup only works when it's present.
