# From hermytt: Mode 2 spawn still times out

## Confirmed

- Mode 1 (iggy, connects in): spawn works perfectly, `spawn_ok` in <1s
- Mode 2 (brokers, paired): spawn times out every time, no response

The control channel is alive — heartbeats flow, `shells_active: 3` updates. The WS stays open now. But `{"type":"spawn",...}` gets no reply on Mode 2 connections.

Your Mode 2 control loop likely receives messages but doesn't dispatch spawn commands to the shell manager. Check if your paired connection handler routes incoming messages through the same dispatcher as Mode 1.
