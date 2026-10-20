//! Foreground game detection and process-exit notifications; no periodic scan.
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::Manager;
use windows::{
    core::{Result, PWSTR},
    Win32::{
        Foundation::{CloseHandle, HANDLE, HWND, WAIT_FAILED, WAIT_OBJECT_0},
        System::Threading::*,
        UI::{Accessibility::*, WindowsAndMessaging::*},
    },
};

type Catalog = HashMap<String, (String, String)>;
pub(crate) struct Service {
    wake: Arc<Shutdown>,
    pub(crate) current: Arc<Mutex<Option<serde_json::Value>>>,
}

impl Drop for Service {
    fn drop(&mut self) {
        // SAFETY: this Service retains shared ownership while signaling shutdown.
        unsafe {
            let _ = SetEvent(HANDLE(self.wake.0 as *mut _));
        }
    }
}

struct Shutdown(usize);
impl Drop for Shutdown {
    fn drop(&mut self) {
        // SAFETY: last shared owner; the monitor and shutdown caller are finished.
        unsafe {
            let _ = CloseHandle(HANDLE(self.0 as *mut _));
        }
    }
}
struct Process(HANDLE);
impl Drop for Process {
    fn drop(&mut self) {
        // SAFETY: owned process handle, closed once on the monitor thread.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
struct Monitor {
    app: tauri::AppHandle,
    catalog: Catalog,
    process: Option<(u32, Process)>,
    current: Arc<Mutex<Option<serde_json::Value>>>,
}
thread_local! { static MONITOR: RefCell<Option<Monitor>> = const { RefCell::new(None) }; }

pub(crate) fn start(app: &tauri::AppHandle) -> Result<()> {
    if !app.state::<crate::mods::Config>().vencord || !app.state::<crate::mods::Config>().activity {
        return Ok(());
    }
    // SAFETY: event ownership transfers to the spawned monitor on success.
    let event = unsafe { CreateEventW(None, true, false, None)? };
    let wake = Arc::new(Shutdown(event.0 as usize));
    let thread_wake = wake.clone();
    let current = Arc::new(Mutex::new(None));
    let state = current.clone();
    let app = app.clone();
    let owner = app.clone();
    let spawn = std::thread::Builder::new()
        .name("game-activity".into())
        .spawn(move || {
            if let Err(error) = run(app, HANDLE(thread_wake.0 as *mut _), state) {
                eprintln!("Game detection unavailable: {error}");
            }
        });
    if let Err(error) = spawn {
        return Err(windows::core::Error::new(
            windows::core::HRESULT(0x80004005u32 as i32),
            error.to_string(),
        ));
    }
    owner.manage(Service { wake, current });
    Ok(())
}

pub(crate) fn stop(app: &tauri::AppHandle) {
    if let Some(service) = app.try_state::<Service>() {
        // SAFETY: Service keeps the shared shutdown event alive.
        unsafe {
            let _ = SetEvent(HANDLE(service.wake.0 as *mut _));
        }
    }
}

fn publish(app: &tauri::AppHandle, activity: Option<serde_json::Value>) {
    if let Some(window) = app.get_webview_window("main") {
        if window
            .url()
            .is_ok_and(|url| crate::navigation::allowed(url.as_str()))
        {
            let payload = activity.unwrap_or(serde_json::Value::Null);
            if let Err(error) = window.eval(format!(
                "window.__discardActivity={payload};window.dispatchEvent(new Event('discard-activity'));"
            )) { eprintln!("Could not update game activity: {error}"); }
        }
    }
}

pub(crate) fn replay(app: &tauri::AppHandle) {
    if let Some(service) = app.try_state::<Service>() {
        let current = match service.current.lock() {
            Ok(current) => current.clone(),
            Err(_) => return,
        };
        publish(app, current);
    }
}

impl Monitor {
    fn changed(&mut self, activity: Option<serde_json::Value>) {
        if let Ok(mut current) = self.current.lock() {
            *current = activity.clone();
        }
        publish(&self.app, activity);
    }
    fn foreground(&mut self, hwnd: HWND) {
        // SAFETY: WinEvent supplies a live or recently destroyed HWND. All failed
        // queries are ignored; process handles remain owned by this thread.
        unsafe {
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 || self.process.as_ref().is_some_and(|(old, _)| *old == pid) {
                return;
            }
            let Ok(handle) = OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION
                    | PROCESS_ACCESS_RIGHTS(SYNCHRONIZATION_SYNCHRONIZE.0),
                false,
                pid,
            ) else {
                return;
            };
            let process = Process(handle);
            let mut exit_code = 0;
            if GetExitCodeProcess(handle, &mut exit_code).is_err() || exit_code != 259 {
                return;
            }
            let mut buffer = [0u16; 32768];
            let mut length = buffer.len() as u32;
            if QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut length,
            )
            .is_err()
            {
                return;
            }
            let path = String::from_utf16_lossy(&buffer[..length as usize])
                .replace('\\', "/")
                .to_lowercase();
            // Match whole path components, most-specific suffix first.
            let mut suffix = path.as_str();
            loop {
                if let Some((id, name)) = self.catalog.get(suffix) {
                    let activity = serde_json::json!({
                        "application_id": id, "name": name, "type": 0,
                        "timestamps": { "start": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64 }
                    });
                    self.process = Some((pid, process));
                    self.changed(Some(activity));
                    return;
                }
                let Some((_, remaining)) = suffix.split_once('/') else {
                    break;
                };
                suffix = remaining;
            }
        }
    }
}

unsafe extern "system" fn foreground_event(
    _: HWINEVENTHOOK,
    _: u32,
    hwnd: HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    MONITOR.with(|slot| {
        if let Ok(mut monitor) = slot.try_borrow_mut() {
            if let Some(monitor) = monitor.as_mut() {
                monitor.foreground(hwnd);
            }
        }
    });
}

fn run(
    app: tauri::AppHandle,
    event: HANDLE,
    current: Arc<Mutex<Option<serde_json::Value>>>,
) -> Result<()> {
    let catalog: Catalog =
        serde_json::from_str(include_str!("../../vendor/games.json")).map_err(|e| {
            windows::core::Error::new(windows::core::HRESULT(0x80004005u32 as i32), e.to_string())
        })?;
    MONITOR.set(Some(Monitor {
        app,
        catalog,
        process: None,
        current,
    }));
    // SAFETY: out-of-context callbacks run only on this thread while its message
    // queue is pumped. The hook and TLS are released before the thread returns.
    unsafe {
        let hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(foreground_event),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        if hook.is_invalid() {
            MONITOR.set(None);
            return Err(windows::core::Error::from_thread());
        }
        MONITOR.with_borrow_mut(|slot| {
            if let Some(monitor) = slot {
                monitor.foreground(GetForegroundWindow());
            }
        });
        let result = loop {
            let process = MONITOR.with_borrow(|slot| {
                slot.as_ref()
                    .and_then(|m| m.process.as_ref().map(|(_, p)| p.0))
            });
            let mut handles = vec![event];
            if let Some(handle) = process {
                handles.push(handle);
            }
            let status = MsgWaitForMultipleObjects(Some(&handles), false, INFINITE, QS_ALLINPUT);
            if status == WAIT_OBJECT_0 {
                break Ok(());
            }
            if status == WAIT_FAILED {
                break Err(windows::core::Error::from_thread());
            }
            if process.is_some() && status.0 == WAIT_OBJECT_0.0 + 1 {
                MONITOR.with_borrow_mut(|slot| {
                    if let Some(m) = slot {
                        m.process = None;
                        m.changed(None);
                        m.foreground(GetForegroundWindow());
                    }
                });
            }
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        };
        let _ = UnhookWinEvent(hook);
        MONITOR.set(None);
        result
    }
}
