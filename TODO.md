# Shytti TODO

## Session persistence across restarts
Survive daemon restarts without losing shells. Needs a small fd-holder process that keeps PTY master fds alive while shytti restarts. Like the one useful part of tmux, but in Rust. Unix doesn't let you re-attach to a closed master fd — so something has to hold it.

Approach: fork a minimal `shytti-keeper` that inherits PTY fds via SCM_RIGHTS, stays alive across shytti restarts, hands them back when shytti comes up. No scrollback replay needed — Hermytt/Crytter handle that.
