//! Real Discord header with native, accessible window buttons; no page bridge.
use std::{cell::Cell, mem::size_of};
use windows::{
    core::w,
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::{
            Dwm::{
                DwmSetWindowAttribute, DWMNCRP_DISABLED, DWMNCRP_USEWINDOWSTYLE,
                DWMWA_NCRENDERING_POLICY,
            },
            Gdi::*,
        },
        UI::{
            Controls::{
                DRAWITEMSTRUCT, ODS_FOCUS, ODS_NOFOCUSRECT, ODS_SELECTED, ODT_BUTTON, WM_MOUSELEAVE,
            },
            HiDpi::GetDpiForWindow,
            Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT},
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::*,
        },
    },
};
const HOOK: usize = 0x44495343;
const STRIP: i32 = 0x4449;
const MIN: i32 = 101;
const MAX: i32 = 102;
const CLOSE: i32 = 103;
pub(super) const PAGE_STYLE: &str = include_str!("titlebar.js");
thread_local! { static HOVER:Cell<i32>=const{Cell::new(0)}; }

pub(super) fn attach(window: &tauri::WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let hwnd = window.hwnd()?;
    // SAFETY: setup and all callbacks run on this HWND's UI thread. Native
    // children belong to it and die with it. Hooks store no borrowed pointers.
    unsafe {
        let strip = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("Discard controls"),
            WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
            0,
            0,
            132,
            32,
            Some(hwnd),
            Some(HMENU(STRIP as *mut _)),
            None,
            None,
        )?;
        let result = (|| -> windows::core::Result<()> {
            SetWindowSubclass(strip, Some(container_proc), HOOK, 0).ok()?;
            for (id, name) in [
                (MIN, w!("Réduire")),
                (MAX, w!("Agrandir ou restaurer")),
                (CLOSE, w!("Fermer vers le tray")),
            ] {
                let button = CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("BUTTON"),
                    name,
                    WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
                    0,
                    0,
                    44,
                    32,
                    Some(strip),
                    Some(HMENU(id as *mut _)),
                    None,
                    None,
                )?;
                SetWindowSubclass(button, Some(button_proc), HOOK, 0).ok()?;
            }
            SetWindowSubclass(hwnd, Some(frame_proc), HOOK, 0).ok()?;
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_NCRENDERING_POLICY,
                &DWMNCRP_DISABLED as *const _ as _,
                size_of::<i32>() as u32,
            )?;
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            )?;
            layout(hwnd);
            Ok(())
        })();
        if let Err(error) = result {
            let _ = RemoveWindowSubclass(hwnd, Some(frame_proc), HOOK);
            let _ = DestroyWindow(strip);
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

/// Keep the asynchronously created Wry host below the caption controls.
pub(super) fn above_webview(host: HWND) {
    // SAFETY: live controller parent on the owning UI STA. Sibling clipping
    // prevents Chromium from painting over the higher native control strip.
    unsafe {
        let style = GetWindowLongPtrW(host, GWL_STYLE);
        SetWindowLongPtrW(host, GWL_STYLE, style | WS_CLIPSIBLINGS.0 as isize);
        layout(GetAncestor(host, GA_ROOT));
    }
}

// SAFETY: registered only on our live UI-thread HWNDs. Windows message pointers
// remain borrowed during synchronous callbacks; default procedures keep ownership.
unsafe extern "system" fn frame_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    id: usize,
    _: usize,
) -> LRESULT {
    unsafe {
        match msg {
            WM_NCCALCSIZE => {
                let rect = if wp.0 != 0 {
                    std::ptr::addr_of_mut!((*(lp.0 as *mut NCCALCSIZE_PARAMS)).rgrc[0])
                } else {
                    lp.0 as *mut RECT
                };
                // Use the entire restored window, without a visible frame.
                if IsZoomed(hwnd).as_bool() {
                    let original = *rect;
                    DefSubclassProc(hwnd, msg, wp, lp);
                    (*rect).top = original.top + ((*rect).left - original.left);
                }
                return LRESULT(0);
            }
            WM_SIZE | WM_DPICHANGED => {
                let result = DefSubclassProc(hwnd, msg, wp, lp);
                layout(hwnd);
                return result;
            }
            WM_NCPAINT | WM_NCACTIVATE => {
                return LRESULT(1);
            }
            WM_NCDESTROY => {
                let _ = RemoveWindowSubclass(hwnd, Some(frame_proc), id);
            }
            _ => {}
        }
        DefSubclassProc(hwnd, msg, wp, lp)
    }
}

unsafe extern "system" fn container_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    id: usize,
    _: usize,
) -> LRESULT {
    unsafe {
        match msg {
            WM_DRAWITEM if lp.0 != 0 => {
                let item = &*(lp.0 as *const DRAWITEMSTRUCT);
                if item.CtlType == ODT_BUTTON {
                    paint_button(hwnd, item);
                    return LRESULT(1);
                }
            }
            WM_COMMAND if (wp.0 >> 16) as u32 == BN_CLICKED => {
                let root = GetAncestor(hwnd, GA_ROOT);
                let command = match (wp.0 & 0xffff) as i32 {
                    MIN => Some(SC_MINIMIZE),
                    MAX => Some(if IsZoomed(root).as_bool() {
                        SC_RESTORE
                    } else {
                        SC_MAXIMIZE
                    }),
                    CLOSE => Some(SC_CLOSE),
                    _ => None,
                };
                if let Some(command) = command {
                    if let Err(error) = PostMessageW(
                        Some(root),
                        WM_SYSCOMMAND,
                        WPARAM(command as usize),
                        LPARAM(0),
                    ) {
                        eprintln!("Window control failed: {error}");
                    } else if matches!(command, SC_MAXIMIZE | SC_RESTORE) {
                        super::windows::focus_content();
                    }
                    return LRESULT(0);
                }
            }
            WM_ERASEBKGND => {
                return LRESULT(1);
            }
            WM_NCDESTROY => {
                let _ = RemoveWindowSubclass(hwnd, Some(container_proc), id);
            }
            _ => {}
        }
        DefSubclassProc(hwnd, msg, wp, lp)
    }
}

unsafe extern "system" fn button_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    id: usize,
    _: usize,
) -> LRESULT {
    unsafe {
        let control = GetDlgCtrlID(hwnd);
        match msg {
            WM_MOUSEMOVE if HOVER.get() != control => {
                HOVER.set(control);
                let mut tracking = TRACKMOUSEEVENT {
                    cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                if let Err(error) = TrackMouseEvent(&mut tracking) {
                    eprintln!("Caption hover tracking failed: {error}");
                }
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            WM_MOUSELEAVE => {
                if HOVER.get() == control {
                    HOVER.set(0);
                }
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            WM_NCDESTROY => {
                if HOVER.get() == control {
                    HOVER.set(0);
                }
                let _ = RemoveWindowSubclass(hwnd, Some(button_proc), id);
            }
            _ => {}
        }
        DefSubclassProc(hwnd, msg, wp, lp)
    }
}
unsafe fn scale(hwnd: HWND, value: i32) -> i32 {
    // SAFETY: caller supplies this thread's live HWND.
    unsafe { (value * GetDpiForWindow(hwnd) as i32 + 48) / 96 }
}
unsafe fn layout(hwnd: HWND) {
    // SAFETY: all native child IDs and handles belong to this UI thread.
    unsafe {
        let Ok(strip) = GetDlgItem(Some(hwnd), STRIP) else {
            return;
        };
        let mut rect = RECT::default();
        if GetClientRect(hwnd, &mut rect).is_err() {
            return;
        }
        let width = scale(hwnd, 44);
        let height = scale(hwnd, 32);
        if let Err(error) = SetWindowPos(
            strip,
            Some(HWND_TOP),
            (rect.right - 3 * width).max(0),
            0,
            3 * width,
            height,
            SWP_NOACTIVATE,
        ) {
            eprintln!("Could not place window controls: {error}");
        }
        for (index, id) in [MIN, MAX, CLOSE].into_iter().enumerate() {
            if let Ok(button) = GetDlgItem(Some(strip), id) {
                let _ = SetWindowPos(
                    button,
                    None,
                    index as i32 * width,
                    0,
                    width,
                    height,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                let _ = InvalidateRect(Some(button), None, false);
            }
        }
    }
}
unsafe fn fill(dc: HDC, rect: &RECT, color: u32) {
    // SAFETY: the temporary brush is not selected and is deleted synchronously.
    unsafe {
        let brush = CreateSolidBrush(COLORREF(color));
        FillRect(dc, rect, brush);
        let _ = DeleteObject(brush.into());
    }
}
unsafe fn paint_button(container: HWND, item: &DRAWITEMSTRUCT) {
    // SAFETY: DRAWITEMSTRUCT/DC are borrowed only for WM_DRAWITEM; the selected
    // GDI pen is restored before deletion. Native BUTTON owns press/focus input.
    unsafe {
        let hot = HOVER.get() == item.CtlID as i32;
        let pressed = item.itemState.0 & ODS_SELECTED.0 != 0;
        let close = item.CtlID as i32 == CLOSE;
        let bg = match (close, pressed, hot) {
            (true, true, _) => 0x332db9,
            (true, false, true) => 0x3c37da,
            (_, true, _) => 0x433c3c,
            (_, false, true) => 0x3b3535,
            _ => 0x141212,
        };
        fill(item.hDC, &item.rcItem, bg);
        let color = if close && (hot || pressed) {
            0xffffff
        } else if hot || pressed {
            0xe1dedb
        } else {
            0x777372
        };
        let pen = CreatePen(PS_SOLID, scale(container, 1).max(1), COLORREF(color));
        if pen.is_invalid() {
            return;
        }
        let old = SelectObject(item.hDC, pen.into());
        let cx = (item.rcItem.left + item.rcItem.right) / 2;
        let cy = (item.rcItem.top + item.rcItem.bottom) / 2;
        let h = scale(container, 5);
        let line = |points: &[(i32, i32)]| {
            let points: Vec<POINT> = points
                .iter()
                .map(|&(x, y)| POINT {
                    x: cx + x,
                    y: cy + y,
                })
                .collect();
            let _ = Polyline(item.hDC, &points);
        };
        match item.CtlID as i32 {
            MIN => line(&[(-h, 0), (h + 1, 0)]),
            MAX if IsZoomed(GetAncestor(container, GA_ROOT)).as_bool() => {
                let d = scale(container, 2);
                line(&[(-h + d, -h), (h, -h), (h, h - d)]);
                line(&[
                    (-h, -h + d),
                    (h - d, -h + d),
                    (h - d, h),
                    (-h, h),
                    (-h, -h + d),
                ]);
            }
            MAX => line(&[(-h, -h), (h, -h), (h, h), (-h, h), (-h, -h)]),
            CLOSE => {
                line(&[(-h, -h), (h + 1, h + 1)]);
                line(&[(-h, h), (h + 1, -h - 1)]);
            }
            _ => {}
        }
        SelectObject(item.hDC, old);
        let _ = DeleteObject(pen.into());
        if item.itemState.0 & ODS_FOCUS.0 != 0 && item.itemState.0 & ODS_NOFOCUSRECT.0 == 0 {
            let mut rect = item.rcItem;
            rect.left += 2;
            rect.top += 2;
            rect.right -= 2;
            rect.bottom -= 2;
            let _ = DrawFocusRect(item.hDC, &rect);
        }
    }
}
