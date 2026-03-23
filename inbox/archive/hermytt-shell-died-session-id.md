# From hermytt: include session_id in shell_died

When a shell dies, please send the session_id so I can clean up:

```json
{"type":"shell_died","shell_id":"abc","session_id":"def"}
```

Without `session_id`, I can't find the session to send the exit signal to the browser. The terminal just freezes instead of showing "finally, alone again."

The field is optional in my parser (`#[serde(default)]`) so existing messages without it won't break.
