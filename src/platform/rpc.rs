//! Minimal local Discord IPC: handshake, ping and SET_ACTIVITY only.
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::Manager;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::{NamedPipeServer, ServerOptions},
    sync::{watch, Semaphore},
};

type Activities = Arc<Mutex<BTreeMap<String, Value>>>;
pub(crate) struct Service {
    cancel: watch::Sender<bool>,
    current: Activities,
}

pub(crate) fn start(app: &tauri::AppHandle) {
    let config = app.state::<crate::mods::Config>();
    if !config.vencord || !config.activity {
        return;
    }
    let (cancel, receiver) = watch::channel(false);
    let current = Arc::new(Mutex::new(BTreeMap::new()));
    app.manage(Service {
        cancel,
        current: current.clone(),
    });
    let limit = Arc::new(Semaphore::new(8));
    for index in 0..10 {
        let (app, current, limit, mut stop) = (
            app.clone(),
            current.clone(),
            limit.clone(),
            receiver.clone(),
        );
        tauri::async_runtime::spawn(async move {
            let path = format!(r"\\.\pipe\discord-ipc-{index}");
            // Do not take over a pipe already owned by Discord or another client.
            let Ok(mut listener) = ServerOptions::new()
                .first_pipe_instance(true)
                .reject_remote_clients(true)
                .max_instances(9)
                .create(&path)
            else {
                return;
            };
            let mut sequence = 0u64;
            loop {
                tokio::select! { _ = stop.changed() => break, result = listener.connect() => if result.is_err() { break; } }
                let Ok(next) = ServerOptions::new()
                    .reject_remote_clients(true)
                    .max_instances(9)
                    .create(&path)
                else {
                    break;
                };
                let connection = std::mem::replace(&mut listener, next);
                let Ok(permit) = limit.clone().try_acquire_owned() else {
                    continue;
                };
                sequence += 1;
                let id = format!("discard-rpc-{index}-{sequence}");
                let (app, current, mut stop) = (app.clone(), current.clone(), stop.clone());
                tauri::async_runtime::spawn(async move {
                    let _permit = permit;
                    tokio::select! { _ = stop.changed() => {}, _ = client(connection, &app, &current, &id) => {} }
                    if let Ok(mut state) = current.lock() {
                        state.remove(&id);
                    }
                    replay(&app);
                });
            }
        });
    }
}

pub(crate) fn stop(app: &tauri::AppHandle) {
    if let Some(service) = app.try_state::<Service>() {
        let _ = service.cancel.send(true);
    }
}

pub(crate) fn replay(app: &tauri::AppHandle) {
    let Some(service) = app.try_state::<Service>() else {
        return;
    };
    // Release the state lock before any synchronous UI-thread dispatch.
    let current = match service.current.lock() {
        Ok(current) => current.clone(),
        Err(_) => return,
    };
    if let Some(window) = app.get_webview_window("main") {
        if window
            .url()
            .is_ok_and(|u| crate::navigation::allowed(u.as_str()))
        {
            if let Err(error) = window.eval(format!(
                "window.__discardRpc={};window.dispatchEvent(new Event('discard-rpc'));",
                json!(current)
            )) {
                eprintln!("Could not publish RPC activity: {error}");
            }
        }
    }
}

async fn read(pipe: &mut NamedPipeServer) -> std::io::Result<(u32, Vec<u8>)> {
    let operation = pipe.read_u32_le().await?;
    let size = pipe.read_u32_le().await? as usize;
    if size > 65536 {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    let mut data = vec![0; size];
    pipe.read_exact(&mut data).await?;
    Ok((operation, data))
}

async fn write(pipe: &mut NamedPipeServer, operation: u32, data: &[u8]) -> std::io::Result<()> {
    pipe.write_u32_le(operation).await?;
    pipe.write_u32_le(data.len() as u32).await?;
    pipe.write_all(data).await
}

async fn client(
    mut pipe: NamedPipeServer,
    app: &tauri::AppHandle,
    current: &Activities,
    id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (operation, data) = tokio::time::timeout(Duration::from_secs(5), read(&mut pipe)).await??;
    let hello: Value = serde_json::from_slice(&data)?;
    let application = hello["client_id"]
        .as_str()
        .filter(|s| (1..=20).contains(&s.len()) && s.bytes().all(|c| c.is_ascii_digit()))
        .ok_or("Invalid client id")?;
    if operation != 0 || hello["v"] != 1 {
        return Err("Unsupported RPC handshake".into());
    }
    let ready = json!({ "cmd":"DISPATCH", "evt":"READY", "nonce":null, "data": {
        "v":1, "config":{"cdn_host":"cdn.discordapp.com", "api_endpoint":"//discord.com/api", "environment":"production"},
        "user":{"id":"1045800378228281345", "username":"Discard", "discriminator":"0", "avatar":null, "bot":false, "flags":0, "premium_type":0}
    }});
    write(&mut pipe, 1, ready.to_string().as_bytes()).await?;
    let mut since = Instant::now();
    let mut requests = 0;
    loop {
        let (operation, data) = read(&mut pipe).await?;
        if since.elapsed() >= Duration::from_secs(1) {
            since = Instant::now();
            requests = 0;
        }
        requests += 1;
        if requests > 60 {
            return Err("RPC rate limit".into());
        }
        match operation {
            2 => return Ok(()),
            3 => {
                write(&mut pipe, 4, &data).await?;
                continue;
            }
            1 => {}
            _ => return Err("Unsupported opcode".into()),
        }
        let request: Value = serde_json::from_slice(&data)?;
        let cmd = request["cmd"].as_str().unwrap_or("");
        let activity = &request["args"]["activity"];
        let response = if cmd == "SET_ACTIVITY" {
            let activity = if activity.is_null() {
                Value::Null
            } else {
                let input = activity.as_object().ok_or("Invalid activity")?;
                let mut filtered = serde_json::Map::new();
                for key in ["name", "state", "details", "assets", "party", "timestamps"] {
                    if let Some(value) = input.get(key) {
                        filtered.insert(key.into(), value.clone());
                    }
                }
                if let Some(times) = filtered
                    .get_mut("timestamps")
                    .and_then(Value::as_object_mut)
                {
                    for value in times.values_mut() {
                        if let Some(number) = value.as_u64() {
                            if number < 100_000_000_000 {
                                *value = json!(number.saturating_mul(1000));
                            }
                        }
                    }
                }
                filtered.insert("application_id".into(), json!(application));
                filtered.insert("type".into(), json!(0));
                Value::Object(filtered)
            };
            if let Ok(mut state) = current.lock() {
                if activity.is_null() {
                    state.remove(id);
                } else {
                    state.insert(id.into(), activity.clone());
                }
            }
            replay(app);
            json!({"cmd":cmd,"evt":null,"nonce":request["nonce"],"data":activity})
        } else {
            json!({"cmd":cmd,"evt":"ERROR","nonce":request["nonce"],"data":{"code":4000,"message":"Only SET_ACTIVITY is supported"}})
        };
        write(&mut pipe, 1, response.to_string().as_bytes()).await?;
    }
}
