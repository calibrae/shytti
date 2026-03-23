mod cli;

use std::sync::Arc;

use cli::Command;
use shytti::{api, bridge, config, control, shell};

#[tokio::main]
async fn main() {
    let cmd = cli::parse();

    match cmd {
        Command::Daemon { config } => {
            tracing_subscriber::fmt().init();

            let cfg = match config::Config::load(config) {
                Ok(c) => c,
                Err(e) => { eprintln!("config error: {e}"); std::process::exit(1); }
            };

            tracing::info!("shytti starting");

            let manager = shell::ShellManager::new();
            let bridge = Arc::new(bridge::HermyttBridge::new(
                &cfg.daemon.hermytt_url,
                &cfg.daemon.hermytt_key,
            ));

            for shell_cfg in &cfg.shells {
                if shell_cfg.autostart {
                    match manager.spawn(shell_cfg.into()).await {
                        Ok(id) => {
                            tracing::info!(name = %shell_cfg.name, %id, "auto-spawned");
                            if let Err(e) = bridge.attach(&id, &manager).await {
                                tracing::error!(%id, "bridge failed: {e}");
                            }
                        }
                        Err(e) => tracing::error!(name = %shell_cfg.name, "spawn failed: {e}"),
                    }
                }
            }

            // Mode 1: connect control WS to Hermytt
            control::connect_to_hermytt(
                &cfg.daemon.hermytt_url,
                &cfg.daemon.hermytt_key,
                manager.clone(),
                bridge.clone(),
            ).await;

            if let Err(e) = api::serve(cfg, manager, bridge).await {
                eprintln!("fatal: {e}");
                std::process::exit(1);
            }
        }
        Command::Pair { config } => {
            tracing_subscriber::fmt().init();

            let cfg = match config::Config::load(config) {
                Ok(c) => c,
                Err(e) => { eprintln!("config error: {e}"); std::process::exit(1); }
            };

            let (token, encoded) = control::PairToken::generate(&cfg.daemon.listen);

            eprintln!("Pairing token (expires in 5 minutes):");
            eprintln!();
            println!("{encoded}");
            eprintln!();
            eprintln!("Paste this token in the Hermytt admin UI to pair.");
            eprintln!("Listening on {}:{} ...", token.ip, token.port);

            // Start daemon with pair state active
            let manager = shell::ShellManager::new();
            let bridge = Arc::new(bridge::HermyttBridge::new(
                &cfg.daemon.hermytt_url,
                &cfg.daemon.hermytt_key,
            ));

            // Set up pair state before serving
            let app = api::router(&cfg, manager.clone(), bridge.clone());

            // We need to inject the pair state. Build state through router, then set pair.
            // Actually, we need access to AppState. Let's build it differently.
            // For now: start serving, the /pair endpoint will validate the token.
            // We need to set pair_state on the AppState. The router creates it internally.
            // Let's restructure: pass pair state through config or a shared ref.

            // Simpler: start daemon normally, set pair state via the router's state.
            // The router returns a Router with AppState. We can't easily reach into it.
            // Instead: start the full daemon and set pair state through api module.

            // Cleanest: serve takes an optional pair token
            let listener = tokio::net::TcpListener::bind(&cfg.daemon.listen).await.unwrap();
            let actual_addr = listener.local_addr().unwrap();
            tracing::info!(addr = %actual_addr, "listening for pairing");

            // Build router and inject pair state
            let pair_state = control::PairState {
                pair_key: token.key.clone(),
                long_lived_key: None,
                used: false,
            };

            let app = api::router(&cfg, manager, bridge);
            // We need to reach into the state... Let's use a different approach.
            // Create state externally and share it.

            // Actually the simplest fix: make router accept optional pair state
            // But that's a bigger refactor. For now, let's have pair just start the daemon
            // with the pair state pre-loaded. We'll add a method to set it.

            // HACK: Start daemon, then set pair state via a channel or shared state.
            // The router's AppState has pair_state: Mutex<Option<PairState>>.
            // We can't reach it after router() returns a Router.

            // Best fix: have router() return (Router, Arc<AppState>) so we can set pair_state.
            drop(app);
            drop(listener);

            serve_with_pair(cfg, pair_state).await;
        }
        Command::Spawn { name, shell, cwd, host, agent, cmd } => {
            let body = serde_json::json!({
                "name": name, "shell": shell, "cwd": cwd,
                "host": host, "agent": agent, "cmd": cmd,
            });
            print_response(http_req("POST", "/shells", Some(&body.to_string())));
        }
        Command::List => {
            print_response(http_req("GET", "/shells", None));
        }
        Command::Kill { id } => {
            print_response(http_req("DELETE", &format!("/shells/{id}"), None));
        }
    }
}

async fn serve_with_pair(cfg: config::Config, pair_state: control::PairState) {
    let manager = shell::ShellManager::new();
    let bridge = Arc::new(bridge::HermyttBridge::new(
        &cfg.daemon.hermytt_url,
        &cfg.daemon.hermytt_key,
    ));

    // Build router to get state handle
    let (app, state) = api::router_with_state(&cfg, manager, bridge);
    *state.pair_state.lock().await = Some(pair_state);

    let listener = tokio::net::TcpListener::bind(&cfg.daemon.listen).await.unwrap();
    tracing::info!(addr = %cfg.daemon.listen, "listening (pair mode)");
    axum::serve(listener, app).await.unwrap();
}

fn http_req(method: &str, path: &str, body: Option<&str>) -> Result<String, String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let mut conn = TcpStream::connect("127.0.0.1:7778")
        .map_err(|e| format!("connect failed (is daemon running?): {e}"))?;

    let body_bytes = body.unwrap_or("");
    let req = if body.is_some() {
        format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body_bytes}",
            body_bytes.len()
        )
    } else {
        format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
    };

    conn.write_all(req.as_bytes()).map_err(|e| e.to_string())?;

    let mut resp = String::new();
    conn.read_to_string(&mut resp).map_err(|e| e.to_string())?;

    match resp.split_once("\r\n\r\n") {
        Some((_, body)) => Ok(body.to_string()),
        None => Ok(resp),
    }
}

fn print_response(res: Result<String, String>) {
    match res {
        Ok(body) => println!("{body}"),
        Err(e) => { eprintln!("error: {e}"); std::process::exit(1); }
    }
}
