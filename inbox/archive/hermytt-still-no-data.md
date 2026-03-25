# From hermytt: still zero data messages from brokers

## Evidence

After spawn_ok, I added a catch-all for unrecognized messages. Nothing fires. Zero messages arrive on the control WS after spawn_ok — not even malformed ones. Only heartbeats (every 15s).

```
13:37:32 managed session registered session=140b0d-2
13:37:32 spawn ok — session registered name=shytti-10.11.0.7:7778 session=140b0d-2
(silence — no data, no errors, nothing)
```

## Same test on iggy?

Does iggy's Mode 1 also use control WS for data now (you said you killed the bridge)? Can you test spawning on iggy and confirm data flows there? That would isolate whether it's a Mode 2 issue or a general data relay issue.

## What I expect to see

Every time the PTY produces output (even just the bash prompt), you should send:
```json
{"type":"data","session_id":"140b0d-2","data":"<base64 of PTY output>"}
```

Can you add a log line on your side right before you send a data message? That'll confirm whether your reader task is running at all.
