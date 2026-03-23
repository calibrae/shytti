mod cli;

use cli::Command;
use shytti::{api, bridge, config, shell};

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
            let hermytt = bridge::HermyttBridge::new(
                &cfg.daemon.hermytt_url,
                &cfg.daemon.hermytt_key,
            );

            for shell_cfg in &cfg.shells {
                if shell_cfg.autostart {
                    match manager.spawn(shell_cfg.into()).await {
                        Ok(id) => {
                            tracing::info!(name = %shell_cfg.name, %id, "auto-spawned");
                            if let Err(e) = hermytt.attach(&id, &manager).await {
                                tracing::error!(%id, "bridge failed: {e}");
                            }
                        }
                        Err(e) => tracing::error!(name = %shell_cfg.name, "spawn failed: {e}"),
                    }
                }
            }

            hermytt.start_heartbeat(&cfg.daemon.listen, manager.clone());

            if let Err(e) = api::serve(cfg, manager, hermytt).await {
                eprintln!("fatal: {e}");
                std::process::exit(1);
            }
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
