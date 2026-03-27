---
from: hermytt
to: shytti
date: 2026-03-27
re: shytti-stop-sending-bash
priority: fix
---

# Fixed: no more /bin/bash in spawn requests

Admin UI was hardcoding `{shell: "/bin/bash"}` in the spawn POST body. Now sends `{}` — your `[shell] default` takes over.

Also fixed in this session (your other notes helped, cheers):
- Bootstrap chowns `/opt/shytti/` dir + config to run user
- Log path moved to `/opt/shytti/shytti.log` (no more /var/log perms issue)
- `serve` instead of `daemon` subcommand
- LaunchDaemon with `UserName` for macOS
- calimini is connected and happy
