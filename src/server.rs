use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::RwLock;

use crate::pool::{Counter, Pool, UuidPool};
use crate::protocol::{Request, Response};
use crate::state::{self, AppState};

/// Returns the socket path: `$XDG_RUNTIME_DIR/envcounter.sock`
fn socket_path() -> Result<PathBuf, std::io::Error> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "XDG_RUNTIME_DIR is not set; is systemd user session active?",
        )
    })?;
    Ok(PathBuf::from(runtime_dir).join("envcounter.sock"))
}

fn handle_request(state: &mut AppState, req: Request) -> Response {
    match req {
        Request::CounterNew {
            name,
            step,
            initial,
        } => {
            if state.pools.contains_key(&name) {
                return Response::error(format!("pool '{}' already exists", name));
            }
            let counter = Counter::new(step.unwrap_or(1), initial.unwrap_or(0));
            state.pools.insert(name, Pool::Counter(counter));
            Response::ok_empty()
        }
        Request::CounterRead { name } => match state.pools.get_mut(&name) {
            Some(Pool::Counter(c)) => Response::ok_value(c.read().into()),
            Some(_) => Response::error(format!("'{}' is not a counter", name)),
            None => Response::error(format!("pool '{}' not found", name)),
        },
        Request::CounterSeek { name } => match state.pools.get(&name) {
            Some(Pool::Counter(c)) => Response::ok_value(c.seek().into()),
            Some(_) => Response::error(format!("'{}' is not a counter", name)),
            None => Response::error(format!("pool '{}' not found", name)),
        },
        Request::CounterRm { name } => match state.pools.remove(&name) {
            Some(_) => Response::ok_empty(),
            None => Response::error(format!("pool '{}' not found", name)),
        },
        Request::CounterRename { name, new_name } => {
            if state.pools.contains_key(&new_name) {
                return Response::error(format!("pool '{}' already exists", new_name));
            }
            match state.pools.remove(&name) {
                Some(pool) => {
                    state.pools.insert(new_name, pool);
                    Response::ok_empty()
                }
                None => Response::error(format!("pool '{}' not found", name)),
            }
        }
        Request::CounterReset { name } => match state.pools.get_mut(&name) {
            Some(Pool::Counter(c)) => {
                c.reset();
                Response::ok_empty()
            }
            Some(_) => Response::error(format!("'{}' is not a counter", name)),
            None => Response::error(format!("pool '{}' not found", name)),
        },
        Request::UuidNew { name } => {
            if state.pools.contains_key(&name) {
                return Response::error(format!("pool '{}' already exists", name));
            }
            state.pools.insert(name, Pool::Uuid(UuidPool::new()));
            Response::ok_empty()
        }
        Request::UuidRead { name } => match state.pools.get(&name) {
            Some(Pool::Uuid(u)) => Response::ok_value(u.read().into()),
            Some(_) => Response::error(format!("'{}' is not a uuid pool", name)),
            None => Response::error(format!("pool '{}' not found", name)),
        },
        Request::UuidRm { name } => match state.pools.get(&name) {
            Some(Pool::Uuid(_)) => {
                state.pools.remove(&name);
                Response::ok_empty()
            }
            Some(_) => Response::error(format!("'{}' is not a uuid pool", name)),
            None => Response::error(format!("pool '{}' not found", name)),
        },
        Request::List { filter_type } => {
            let list: serde_json::Value = state
                .pools
                .iter()
                .filter(|(_, pool)| {
                    filter_type
                        .as_deref()
                        .map_or(true, |ft| pool.type_name() == ft)
                })
                .map(|(name, pool)| match pool {
                    Pool::Counter(c) => serde_json::json!({
                        "name": name,
                        "type": "counter",
                        "value": c.value,
                        "step": c.step,
                        "initial": c.initial,
                    }),
                    Pool::Uuid(_) => serde_json::json!({
                        "name": name,
                        "type": "uuid",
                    }),
                })
                .collect();
            Response::ok_value(list)
        }
    }
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let sock_path = socket_path()?;

    // Remove stale socket if it exists
    if sock_path.exists() {
        std::fs::remove_file(&sock_path)?;
    }

    // Load persisted state
    let app_state = state::load()?;
    eprintln!(
        "loaded {} pool(s) from disk",
        app_state.pools.len()
    );
    let shared_state = Arc::new(RwLock::new(app_state));

    let listener = UnixListener::bind(&sock_path)?;
    eprintln!("listening on {}", sock_path.display());

    // Signal handling for graceful shutdown
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    loop {
        tokio::select! {
            accept = listener.accept() => {
                let (stream, _) = accept?;
                let state = Arc::clone(&shared_state);
                tokio::spawn(handle_connection(stream, state));
            }
            _ = sigterm.recv() => {
                eprintln!("received SIGTERM, shutting down...");
                break;
            }
            _ = sigint.recv() => {
                eprintln!("received SIGINT, shutting down...");
                break;
            }
        }
    }

    // Persist state before exit
    let app_state = shared_state.read().await;
    state::save(&app_state)?;
    eprintln!("persisted {} pool(s) to disk", app_state.pools.len());

    // Clean up socket
    let _ = std::fs::remove_file(&sock_path);

    Ok(())
}

async fn handle_connection(stream: tokio::net::UnixStream, state: Arc<RwLock<AppState>>) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("read error: {}", e);
                break;
            }
        }

        let response = match serde_json::from_str::<Request>(line.trim()) {
            Ok(req) => {
                let mut state = state.write().await;
                handle_request(&mut state, req)
            }
            Err(e) => Response::error(format!("invalid request: {}", e)),
        };

        let mut resp_bytes = serde_json::to_vec(&response).unwrap();
        resp_bytes.push(b'\n');

        if let Err(e) = writer.write_all(&resp_bytes).await {
            eprintln!("write error: {}", e);
            break;
        }
    }
}
