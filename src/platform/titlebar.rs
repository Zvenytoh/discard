//! Windows non-client title bar: native sizing, dragging and system commands.
//! No extra browser surface or remote-content privileges.

use std::cell::Cell;
use std::mem::size_of;
use windows::{
    core::w,
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::{
            Dwm::{
                DwmSetWindowAttribute, DWMNCRP_DISABLED, DWMNCRP_USEWINDOWSTYLE,
                DWMWA_NCRENDERING_POLICY,
            },
            Gdi::*,
        },
        UI::{
            HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi},
            Input::KeyboardAndMouse::{ReleaseCapture, SetCapture},
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass, ShellExecuteW},
            WindowsAndMessaging::*,
        },
    },
};

const SUBCLASS: usize = 0x44495343;

// Keep one caption row: Discord's own top bar otherwise duplicates our native
// controls. Static CSS also matches later SPA renders without an observer/timer.
pub(super) const PAGE_STYLE: &str = r#"
(() => {
    if (window.top !== window) return;
    const install = () => {
        if (document.getElementById('discard-single-titlebar')) return;
        const style = document.createElement('style');
        style.id = 'discard-single-titlebar';
        style.textContent = `
            #app-mount div[class^="base_"]:has(> div[class^="bar_"]) {
                grid-template-rows: [top] 0px [titleBarEnd] min-content [noticeEnd] 1fr [contentEnd] min-content [end] !important;
            }
            #app-mount div[class^="base_"] > div[class^="bar_"] {
                display: none !important;
            }
        `;
        document.documentElement.appendChild(style);
    };
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', install, { once: true });
    } else {
        install();
    }
})();
"#;
thread_local! {
    // This adapter owns exactly one top-level window, on the UI thread.
    static PRESSED: Cell<u32> = const { Cell::new(0) };
}

pub(super) fn attach(window: &tauri::WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let hwnd = window.hwnd()?;
    // SAFETY: called on Tauri's owning UI thread with its live HWND. The subclass
    // holds no pointers/state and removes itself at WM_NCDESTROY.
    unsafe {
        SetWindowSubclass(hwnd, Some(procedure), SUBCLASS, 0).ok()?;
        let result = DwmSetWindowAttribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY,
            &DWMNCRP_DISABLED as *const _ as _,
            size_of::<i32>() as u32,
        )
        .and_then(|()| {
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            )
        });
        if let Err(error) = result {
            RemoveWindowSubclass(hwnd, Some(procedure), SUBCLASS).ok()?;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_NCRENDERING_POLICY,
                &DWMNCRP_USEWINDOWSTYLE as *const _ as _,
                size_of::<i32>() as u32,
            );
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
            return Err(error.into());
        }
    }
    Ok(())
}

// SAFETY: Windows invokes this only for the registered HWND on its owning UI
// thread. Message-specific pointers below are valid only during the call; no
// borrowed pointer, device context or GDI object escapes the callback.
unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    id: usize,
    _: usize,
) -> LRESULT {
    unsafe {
        match message {
            WM_NCCALCSIZE => {
                let rect = if wp.0 != 0 {
                    std::ptr::addr_of_mut!((*(lp.0 as *mut NCCALCSIZE_PARAMS)).rgrc[0])
                } else {
                    lp.0 as *mut RECT
                };
                let original = *rect;
                DefSubclassProc(hwnd, message, wp, lp);
                let rect = &mut *rect;
                let border = rect.left - original.left;
                rect.top = original.top + border + scale(hwnd, 32);
                return LRESULT(0);
            }
            WM_NCHITTEST => {
                let mut bounds = RECT::default();
                if GetWindowRect(hwnd, &mut bounds).is_ok() {
                    let x = (lp.0 as i16) as i32 - bounds.left;
                    let y = ((lp.0 >> 16) as i16) as i32 - bounds.top;
                    let border = border(hwnd);
                    let right = bounds.right - bounds.left - border;
                    if x >= border && x < right && y >= border && y < border + scale(hwnd, 32) {
                        let button = scale(hwnd, 44);
                        let hit = if x >= right - button {
                            HTCLOSE
                        } else if x >= right - 2 * button {
                            HTMAXBUTTON
                        } else if x >= right - 3 * button {
                            HTMINBUTTON
                        } else if x >= right - 4 * button {
                            HTHELP
                        } else if x >= right - 5 * button {
                            HTOBJECT
                        } else {
                            HTCAPTION
                        };
                        return LRESULT(hit as isize);
                    }
                }
            }
            WM_NCLBUTTONDOWN => {
                if matches!(
                    wp.0 as u32,
                    HTCLOSE | HTMINBUTTON | HTMAXBUTTON | HTHELP | HTOBJECT
                ) {
                    PRESSED.set(wp.0 as u32);
                    SetCapture(hwnd);
                    return LRESULT(0);
                }
            }
            WM_LBUTTONUP if PRESSED.get() != 0 => {
                let pressed = PRESSED.replace(0);
                let _ = ReleaseCapture();
                let mut cursor = windows::Win32::Foundation::POINT::default();
                if GetCursorPos(&mut cursor).is_err() {
                    return LRESULT(0);
                }
                let position = ((cursor.y as u32 & 0xffff) << 16) | (cursor.x as u32 & 0xffff);
                let hit = SendMessageW(
                    hwnd,
                    WM_NCHITTEST,
                    Some(WPARAM(0)),
                    Some(LPARAM(position as isize)),
                )
                .0 as u32;
                if hit != pressed {
                    return LRESULT(0);
                }
                if pressed == HTOBJECT {
                    super::windows::open_inbox();
                    return LRESULT(0);
                }
                if pressed == HTHELP {
                    let result = ShellExecuteW(
                        Some(hwnd),
                        w!("open"),
                        w!("https://support.discord.com/"),
                        None,
                        None,
                        SW_SHOWNORMAL,
                    );
                    if result.0 as isize <= 32 {
                        eprintln!("Could not open Discord help");
                    }
                    return LRESULT(0);
                }
                let command = match pressed {
                    HTCLOSE => Some(SC_CLOSE),
                    HTMINBUTTON => Some(SC_MINIMIZE),
                    HTMAXBUTTON => Some(if IsZoomed(hwnd).as_bool() {
                        SC_RESTORE
                    } else {
                        SC_MAXIMIZE
                    }),
                    _ => None,
                };
                if let Some(command) = command {
                    if let Err(error) = PostMessageW(
                        Some(hwnd),
                        WM_SYSCOMMAND,
                        WPARAM(command as usize),
                        LPARAM(0),
                    ) {
                        eprintln!("Window command failed: {error}");
                    }
                    return LRESULT(0);
                }
            }
            WM_CAPTURECHANGED => {
                PRESSED.set(0);
            }
            WM_NCPAINT | WM_NCACTIVATE => {
                paint(hwnd);
                return LRESULT(1);
            }
            WM_SETTEXT | WM_SIZE | WM_DPICHANGED => {
                let result = DefSubclassProc(hwnd, message, wp, lp);
                paint(hwnd);
                return result;
            }
            WM_NCDESTROY => {
                let _ = RemoveWindowSubclass(hwnd, Some(procedure), id);
            }
            _ => {}
        }
        DefSubclassProc(hwnd, message, wp, lp)
    }
}

unsafe fn scale(hwnd: HWND, value: i32) -> i32 {
    // SAFETY: caller holds the live UI-thread HWND.
    unsafe { value * GetDpiForWindow(hwnd) as i32 / 96 }
}

unsafe fn border(hwnd: HWND) -> i32 {
    // SAFETY: caller holds the live UI-thread HWND; metrics are queried for its DPI.
    unsafe {
        let dpi = GetDpiForWindow(hwnd);
        GetSystemMetricsForDpi(SM_CXSIZEFRAME, dpi) + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi)
    }
}

unsafe fn paint(hwnd: HWND) {
    // SAFETY: all native handles are used synchronously on their owning thread.
    // The DC is released and every created/selected GDI resource is restored.
    unsafe {
        let mut bounds = RECT::default();
        if GetWindowRect(hwnd, &mut bounds).is_err() {
            return;
        }
        let dc = GetWindowDC(Some(hwnd));
        if dc.is_invalid() {
            return;
        }
        let border = border(hwnd);
        let width = bounds.right - bounds.left;
        let mut bar = RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: border + scale(hwnd, 32),
        };
        let brush = CreateSolidBrush(COLORREF(0x00141212)); // screenshot: #121214
        let height = bounds.bottom - bounds.top;
        ExcludeClipRect(
            dc,
            border,
            border + scale(hwnd, 32),
            width - border,
            height - border,
        );
        let frame = RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        };
        FillRect(dc, &frame, brush);
        let _ = DeleteObject(brush.into());
        SetBkMode(dc, TRANSPARENT);
        SetTextColor(dc, COLORREF(0x00dedede));
        let font = CreateFontW(
            -scale(hwnd, 13),
            0,
            0,
            0,
            500,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            0,
            w!("Segoe UI"),
        );
        if font.is_invalid() {
            ReleaseDC(Some(hwnd), dc);
            return;
        }
        let old = SelectObject(dc, font.into());
        bar.top = border;
        bar.left = border + 5 * scale(hwnd, 44);
        bar.right = width - border - 5 * scale(hwnd, 44);
        let mut title = [0u16; 256];
        let length = GetWindowTextW(hwnd, &mut title) as usize;
        DrawTextW(
            dc,
            &mut title[..length],
            &mut bar,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
        let icons = CreateFontW(
            -scale(hwnd, 12),
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            0,
            w!("Segoe MDL2 Assets"),
        );
        if !icons.is_invalid() {
            SelectObject(dc, icons.into());
        }
        for (index, label) in [
            "\u{E715}",
            "\u{E897}",
            "\u{E921}",
            if IsZoomed(hwnd).as_bool() {
                "\u{E923}"
            } else {
                "\u{E922}"
            },
            "\u{E8BB}",
        ]
        .iter()
        .enumerate()
        {
            let mut rect = RECT {
                left: width - border - (5 - index as i32) * scale(hwnd, 44),
                right: width - border - (4 - index as i32) * scale(hwnd, 44),
                top: border,
                bottom: border + scale(hwnd, 32),
            };
            let mut text: Vec<u16> = label.encode_utf16().collect();
            DrawTextW(
                dc,
                &mut text,
                &mut rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );
        }
        SelectObject(dc, old);
        if !icons.is_invalid() {
            let _ = DeleteObject(icons.into());
        }
        let _ = DeleteObject(font.into());
        ReleaseDC(Some(hwnd), dc);
    }
}
