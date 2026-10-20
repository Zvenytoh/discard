//! Experimental application-audio capture, armed only from the native tray.
use futures_util::{SinkExt, StreamExt};
use std::{collections::VecDeque, sync::Mutex};
use tauri::Manager;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::{
    self,
    handshake::server::{Request, Response},
    Message,
};
use windows::{
    core::BOOL,
    Win32::{
        Foundation::{CloseHandle, FILETIME, HANDLE, HWND, LPARAM, WAIT_OBJECT_0},
        Security::Cryptography::{BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG},
        System::Threading::*,
        UI::WindowsAndMessaging::*,
    },
};

pub(crate) struct Target {
    pub(crate) pid: u32,
    pub(crate) created: u64,
    pub(crate) title: String,
}
struct Session {
    endpoint: String,
    cancel: watch::Sender<bool>,
}
#[derive(Default)]
pub(crate) struct Service(Mutex<Option<Session>>);

pub(crate) fn initialize(app: &tauri::AppHandle) {
    app.manage(Service::default());
}

fn creation_time(handle: HANDLE) -> windows::core::Result<u64> {
    let (mut created, mut exited, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    // SAFETY: caller owns a queryable process handle; output pointers are local.
    unsafe {
        GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user)?;
    }
    Ok(((created.dwHighDateTime as u64) << 32) | created.dwLowDateTime as u64)
}

fn ancestors() -> windows::core::Result<Vec<u32>> {
    use windows::Win32::System::Diagnostics::ToolHelp::*;
    // SAFETY: snapshot is owned here; PROCESSENTRY32W has the required size.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut parents = std::collections::HashMap::new();
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                parents.insert(entry.th32ProcessID, entry.th32ParentProcessID);
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        } else {
            let error = windows::core::Error::from_thread();
            let _ = CloseHandle(snapshot);
            return Err(error);
        }
        let _ = CloseHandle(snapshot);
        let mut result = vec![GetCurrentProcessId()];
        while let Some(parent) = parents.get(result.last().expect("current PID")) {
            if *parent == 0 || result.contains(parent) {
                break;
            }
            result.push(*parent);
        }
        Ok(result)
    }
}

pub(crate) fn targets() -> windows::core::Result<Vec<Target>> {
    unsafe extern "system" fn enumerate(hwnd: HWND, data: LPARAM) -> BOOL {
        // SAFETY: EnumWindows is synchronous; data borrows the caller's Vec.
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() || GetWindowTextLengthW(hwnd) == 0 {
                return BOOL(1);
            }
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == GetCurrentProcessId() || pid == 0 {
                return BOOL(1);
            }
            let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                return BOOL(1);
            };
            let created = creation_time(process);
            let _ = CloseHandle(process);
            let mut title = [0u16; 256];
            let length = GetWindowTextW(hwnd, &mut title);
            if let Ok(created) = created {
                let targets = &mut *(data.0 as *mut Vec<Target>);
                if !targets.iter().any(|t| t.pid == pid) && length > 0 {
                    targets.push(Target {
                        pid,
                        created,
                        title: String::from_utf16_lossy(&title[..length as usize])
                            .replace('&', "&&"),
                    });
                }
            }
            BOOL(1)
        }
    }
    let mut result: Vec<Target> = Vec::new();
    // SAFETY: callback uses this Vec only during the synchronous enumeration.
    unsafe {
        EnumWindows(
            Some(enumerate),
            LPARAM(&mut result as *mut Vec<Target> as isize),
        )?;
    }
    // Excluding Discard's ancestors also excludes their entire process trees,
    // which could otherwise capture incoming call audio and cause an echo.
    let ancestors = ancestors()?;
    result.retain(|target| !ancestors.contains(&target.pid));
    result.sort_by(|a, b| a.title.cmp(&b.title));
    result.truncate(40);
    Ok(result)
}

pub(crate) fn stop(app: &tauri::AppHandle) {
    if let Some(service) = app.try_state::<Service>() {
        if let Ok(mut session) = service.0.lock() {
            if let Some(session) = session.take() {
                let _ = session.cancel.send(true);
            }
        }
    }
    replay(app);
}

pub(crate) fn replay(app: &tauri::AppHandle) {
    let endpoint = app.try_state::<Service>().and_then(|s| {
        s.0.lock()
            .ok()
            .and_then(|s| s.as_ref().map(|s| s.endpoint.clone()))
    });
    if let Some(window) = app.get_webview_window("main") {
        if window
            .url()
            .is_ok_and(|u| crate::navigation::allowed(u.as_str()))
        {
            if let Err(error) = window.eval(format!(
                "window.__discardAudioEndpoint={};",
                serde_json::json!(endpoint)
            )) {
                eprintln!("Could not configure application audio: {error}");
            }
        }
    }
}

pub(crate) fn select(
    app: &tauri::AppHandle,
    pid: u32,
    created: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate native menu data again; reject our own audio and PID reuse.
    if !targets()?
        .iter()
        .any(|t| t.pid == pid && t.created == created)
    {
        return Err("The selected application has closed; refresh the audio menu.".into());
    }
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    listener.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();
    let mut random = [0u8; 32];
    // SAFETY: OS RNG writes only into this owned array.
    unsafe {
        BCryptGenRandom(None, &mut random, BCRYPT_USE_SYSTEM_PREFERRED_RNG).ok()?;
    }
    let token: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let endpoint = format!("ws://127.0.0.1:{port}/{token}");
    let (cancel, receiver) = watch::channel(false);
    let service = app.state::<Service>();
    let mut state = service.0.lock().map_err(|_| "Audio state unavailable")?;
    if let Some(old) = state.take() {
        let _ = old.cancel.send(true);
    }
    *state = Some(Session { endpoint, cancel });
    drop(state);
    let owner = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = serve(listener, token, pid, created, receiver).await {
            eprintln!("Application audio stopped: {error}");
            crate::platform::show_error(&format!("Application audio unavailable: {error}"));
        }
        drop(owner);
    });
    replay(app);
    Ok(())
}

enum Frame {
    Ready,
    Pcm(Vec<u8>),
    Failed(String),
}
// Kernel events support cross-thread signaling; the numeric handle is never exposed to the page.
struct StopEvent(usize);
impl Drop for StopEvent {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(HANDLE(self.0 as *mut _));
        }
    }
}
impl StopEvent {
    fn signal(&self) {
        unsafe {
            let _ = SetEvent(HANDLE(self.0 as *mut _));
        }
    }
}

struct Authorize(String);
impl tungstenite::handshake::server::Callback for Authorize {
    fn on_request(
        self,
        request: &Request,
        response: Response,
    ) -> std::result::Result<Response, tungstenite::handshake::server::ErrorResponse> {
        if request
            .uri()
            .path_and_query()
            .is_some_and(|p| p.as_str() == self.0)
            && request
                .headers()
                .get("origin")
                .is_some_and(|o| o == "https://discord.com")
        {
            Ok(response)
        } else {
            let mut denied = tungstenite::http::Response::new(Some("Forbidden".to_owned()));
            *denied.status_mut() = tungstenite::http::StatusCode::FORBIDDEN;
            Err(denied)
        }
    }
}

async fn serve(
    listener: std::net::TcpListener,
    token: String,
    pid: u32,
    created: u64,
    mut cancel: watch::Receiver<bool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = tokio::net::TcpListener::from_std(listener)?;
    loop {
        let (stream, _) = tokio::select! {
            _ = cancel.changed() => return Ok(()),
            accepted = listener.accept() => accepted?,
        };
        let accept = tokio_tungstenite::accept_hdr_async_with_config(
            stream,
            Authorize(format!("/{token}")),
            Some(
                tungstenite::protocol::WebSocketConfig::default()
                    .max_message_size(Some(1024))
                    .max_frame_size(Some(1024)),
            ),
        );
        let socket = tokio::select! {
            _ = cancel.changed() => return Ok(()),
            result = tokio::time::timeout(std::time::Duration::from_secs(3), accept) => match result { Ok(Ok(s)) => s, _ => continue },
        };
        // One consumer at a time. A failed handshake never starts WASAPI.
        let stop = std::sync::Arc::new(StopEvent(unsafe {
            CreateEventW(None, true, false, None)?.0 as usize
        }));
        let capture_stop = stop.clone();
        let (send, mut receive) = mpsc::channel(4);
        let capture = tauri::async_runtime::spawn_blocking(move || {
            if let Err(error) = capture(pid, created, &capture_stop, &send) {
                eprintln!("Native audio capture failed: {error}");
                let _ = send.try_send(Frame::Failed(error.to_string()));
            }
        });
        let (mut writer, mut reader) = socket.split();
        loop {
            tokio::select! {
                _ = cancel.changed() => break,
                message = reader.next() => match message {
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {},
                    _ => break,
                },
                frame = receive.recv() => {
                    let message = match frame {
                        Some(Frame::Ready) => Message::Text("ready".into()),
                        Some(Frame::Pcm(bytes)) => Message::Binary(bytes.into()),
                        Some(Frame::Failed(error)) => { let _ = writer.send(Message::Text(format!("error:{error}").into())).await; break; },
                        None => break,
                    };
                    let sent = tokio::select! {
                        _ = cancel.changed() => break,
                        sent = tokio::time::timeout(std::time::Duration::from_secs(2), writer.send(message)) => sent,
                    };
                    if !matches!(sent, Ok(Ok(()))) { break; }
                }
            }
        }
        stop.signal();
        if let Err(error) = capture.await {
            eprintln!("Audio worker failed: {error}");
        }
        drop(writer);
        if *cancel.borrow() {
            return Ok(());
        }
    }
}

fn capture(
    pid: u32,
    created: u64,
    stop: &StopEvent,
    send: &mpsc::Sender<Frame>,
) -> Result<(), Box<dyn std::error::Error>> {
    use wasapi::{AudioClient, Direction, SampleType, StreamMode, WaveFormat};
    // SAFETY: the handle is owned here, used only for identity and exit checks.
    let process = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION
                | PROCESS_ACCESS_RIGHTS(SYNCHRONIZATION_SYNCHRONIZE.0),
            false,
            pid,
        )?
    };
    struct OwnedProcess(HANDLE);
    impl Drop for OwnedProcess {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    let process = OwnedProcess(process);
    if creation_time(process.0)? != created {
        return Err("Selected application changed; select it again.".into());
    }
    wasapi::initialize_mta().ok()?;
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            wasapi::deinitialize();
        }
    }
    let _apartment = Apartment;
    let mut client = AudioClient::new_application_loopback_client(pid, true)?;
    client.initialize_client(
        &WaveFormat::new(32, 32, &SampleType::Float, 48000, 2, None),
        &Direction::Capture,
        &StreamMode::EventsShared {
            autoconvert: true,
            buffer_duration_hns: 0,
        },
    )?;
    let event = client.set_get_eventhandle()?;
    // Read packets directly to handle AUDCLNT_BUFFERFLAGS_SILENT safely;
    // WASAPI may return a null data pointer for a silent packet.
    let capture = unsafe {
        client
            .as_iaudioclient()
            .GetService::<windows::Win32::Media::Audio::IAudioCaptureClient>()?
    };
    client.start_stream()?;
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        send.blocking_send(Frame::Ready)?;
        let mut queue = VecDeque::new();
        loop {
            // SAFETY: all handles remain alive for this blocking wait. No polling
            // or native capture occurs when no browser share is connected.
            let status = unsafe {
                WaitForMultipleObjects(
                    &[HANDLE(stop.0 as *mut _), process.0, event.as_handle()],
                    false,
                    INFINITE,
                )
            };
            if status == WAIT_OBJECT_0 || status.0 == WAIT_OBJECT_0.0 + 1 {
                break;
            }
            if status.0 != WAIT_OBJECT_0.0 + 2 {
                return Err("Audio event wait failed".into());
            }
            while read_packet(&capture, &mut queue)? {
                // Bound pending audio to ~170ms, preferring current samples.
                if queue.len() > 8192 * 8 {
                    queue.drain(..queue.len() - 8192 * 8);
                }
                while queue.len() >= 1024 * 8 {
                    let chunk: Vec<u8> = queue.drain(..1024 * 8).collect();
                    match send.try_send(Frame::Pcm(chunk)) {
                        Ok(()) | Err(mpsc::error::TrySendError::Full(_)) => {}
                        Err(mpsc::error::TrySendError::Closed(_)) => return Ok(()),
                    }
                }
            }
        }
        Ok(())
    })();
    client.stop_stream()?;
    result
}

fn read_packet(
    capture: &windows::Win32::Media::Audio::IAudioCaptureClient,
    queue: &mut VecDeque<u8>,
) -> windows::core::Result<bool> {
    use windows::Win32::Media::Audio::AUDCLNT_BUFFERFLAGS_SILENT;
    // SAFETY: interface and packet stay on the owning capture MTA. GetBuffer
    // borrows frames until ReleaseBuffer. Silent packets need no data pointer;
    // non-silent data is copied before release, and only the latest 8192 frames
    // are retained. Every successfully acquired packet is released even on error.
    unsafe {
        if capture.GetNextPacketSize()? == 0 {
            return Ok(false);
        }
        let (mut data, mut frames, mut flags) = (std::ptr::null_mut(), 0u32, 0u32);
        capture.GetBuffer(&mut data, &mut frames, &mut flags, None, None)?;
        let keep = frames.min(8192) as usize;
        let copied = if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 {
            queue.resize(queue.len() + keep * 8, 0);
            Ok(())
        } else if keep == 0 {
            Ok(())
        } else if data.is_null() {
            Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                0x80004003u32 as i32,
            )))
        } else {
            let skip = (frames as usize - keep) * 8;
            queue.extend(std::slice::from_raw_parts(data.add(skip), keep * 8));
            Ok(())
        };
        let released = capture.ReleaseBuffer(frames);
        copied?;
        released?;
        Ok(frames > 0)
    }
}
