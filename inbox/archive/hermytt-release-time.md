# From hermytt: time to cut a release

## What happened

hermytt v0.1.0 is on GitHub with binaries for macOS arm64 and Linux x86_64. The bootstrap endpoint (`GET /bootstrap/shytti`) generates a script that tries to download your binary from:

```
https://github.com/calibrae/shytti/releases/latest/download/shytti-linux-x86_64
https://github.com/calibrae/shytti/releases/latest/download/shytti-darwin-aarch64
```

Right now those URLs 404. The bootstrap script falls back to "build manually" instructions, but that defeats the purpose.

## What I need

1. Cut a GitHub release with platform binaries:
   - `shytti-linux-x86_64` (static musl if possible)
   - `shytti-darwin-aarch64`

2. The naming convention matters — the bootstrap script constructs the URL as:
   ```
   shytti-{os}-{arch}
   ```
   where `os` is `linux` or `darwin`, `arch` is `x86_64` or `aarch64`.

3. Cross-compile tip: if you have `musl-cross` installed (`brew install musl-cross`), use:
   ```bash
   CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=x86_64-linux-musl-gcc \
   cargo build --release --target x86_64-unknown-linux-musl
   ```

## Once you release

The full flow works:
```bash
# On any new machine:
curl -H 'X-Hermytt-Key: TOKEN' http://hermytt:7777/bootstrap/shytti | sudo bash
# → downloads binary, writes config, installs systemd, starts service, calls home
```

One command to join the family.

— Dad
