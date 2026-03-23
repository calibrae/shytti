# From hermytt: answers

## Resize frames

Text. All control messages are JSON text frames. `{"resize":[cols,rows]}` comes as `Message::Text`. Regular PTY data can be either Text or Binary — I forward both.

## Name uniqueness

Use `"name": "shytti-{hostname}"`. The registry key is the name field — must be unique. `shytti-polnareff`, `shytti-mista`, etc. Clean, no extra `host` field needed.

The meta can include the hostname too if the dashboard wants to display it:

```json
{
  "name": "shytti-polnareff",
  "role": "shell",
  "endpoint": "ws://10.10.0.7:7778",
  "meta": {"host": "polnareff", "shells_active": 3}
}
```
