// reparent-spike — vstabs feasibility check
//
// Goal: answer one question — can we SetParent a running VS Code window
// into a container window without breaking IME / GPU rendering / focus?
//
// Flow:
//   1. Open container window (winit, ~960x720)
//   2. Wait 2s, then enumerate top-level windows for ones whose title ends
//      with " - Visual Studio Code"
//   3. Pick the first match, SetParent it into our window, resize to fill
//      client area (with 30px top reserved as fake "tab bar" region)
//   4. Run normal event loop. User manually checks: Korean IME, mouse, focus,
//      moving the container, closing VS Code, etc.
//
// On exit (window close), restore VS Code as a top-level window so we don't
// orphan it inside our container.

use std::time::{Duration, Instant};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use windows::core::PWSTR;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, TRUE};

fn window_hwnd(win: &Window) -> HWND {
    let handle = win
        .window_handle()
        .expect("window handle unavailable")
        .as_raw();
    match handle {
        RawWindowHandle::Win32(h) => HWND(h.hwnd.get() as *mut _),
        other => panic!("expected Win32 window handle, got {other:?}"),
    }
}
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClientRect, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
    MoveWindow, SetParent, SetWindowLongPtrW, ShowWindow, GWL_STYLE, SW_SHOW, WS_CAPTION,
    WS_CHILD, WS_OVERLAPPEDWINDOW, WS_POPUP, WS_THICKFRAME, WS_VISIBLE,
};

const TAB_BAR_HEIGHT: i32 = 30;
const VSCODE_TITLE_SUFFIX: &str = " - Visual Studio Code";

struct App {
    window: Option<Window>,
    vscode_hwnd: Option<HWND>,
    original_style: Option<isize>,
    spawn_time: Instant,
    attempted: bool,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            vscode_hwnd: None,
            original_style: None,
            spawn_time: Instant::now(),
            attempted: false,
        }
    }

    fn try_attach(&mut self) {
        if self.attempted {
            return;
        }
        if self.spawn_time.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.attempted = true;

        let Some(win) = self.window.as_ref() else {
            return;
        };
        let container_hwnd = window_hwnd(win);

        let vscode_hwnd = match find_vscode_window() {
            Some(h) => h,
            None => {
                eprintln!(
                    "[spike] No VS Code window found. Open VS Code (any project) and re-run."
                );
                return;
            }
        };

        unsafe {
            let original = SetWindowLongPtrW(vscode_hwnd, GWL_STYLE, 0);
            self.original_style = Some(original);

            let new_style = (WS_CHILD.0 | WS_VISIBLE.0) as isize;
            SetWindowLongPtrW(vscode_hwnd, GWL_STYLE, new_style);

            match SetParent(vscode_hwnd, container_hwnd) {
                Ok(_) => {
                    println!("[spike] SetParent succeeded. hwnd={:?}", vscode_hwnd);
                    self.vscode_hwnd = Some(vscode_hwnd);
                    let _ = ShowWindow(vscode_hwnd, SW_SHOW);
                    self.layout_child();
                }
                Err(e) => {
                    eprintln!("[spike] SetParent failed: {e:?}");
                    SetWindowLongPtrW(vscode_hwnd, GWL_STYLE, original);
                }
            }
        }
    }

    fn layout_child(&self) {
        let (Some(win), Some(child)) = (self.window.as_ref(), self.vscode_hwnd) else {
            return;
        };
        let container_hwnd = window_hwnd(win);
        unsafe {
            let mut rect = RECT::default();
            if GetClientRect(container_hwnd, &mut rect).is_ok() {
                let w = rect.right - rect.left;
                let h = rect.bottom - rect.top - TAB_BAR_HEIGHT;
                let _ = MoveWindow(child, 0, TAB_BAR_HEIGHT, w, h, true);
            }
        }
    }

    fn detach_on_exit(&mut self) {
        let Some(child) = self.vscode_hwnd.take() else {
            return;
        };
        unsafe {
            let _ = SetParent(child, HWND(std::ptr::null_mut()));
            if let Some(orig) = self.original_style.take() {
                SetWindowLongPtrW(child, GWL_STYLE, orig);
            } else {
                let style =
                    (WS_OVERLAPPEDWINDOW.0 | WS_VISIBLE.0 | WS_CAPTION.0 | WS_THICKFRAME.0)
                        as isize
                        & !(WS_CHILD.0 | WS_POPUP.0) as isize;
                SetWindowLongPtrW(child, GWL_STYLE, style);
            }
            let _ = ShowWindow(child, SW_SHOW);
        }
        println!("[spike] Detached VS Code back to top-level.");
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("vstabs reparent spike — close to detach")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0));
        let window = event_loop
            .create_window(attrs)
            .expect("create window failed");
        self.window = Some(window);
        self.spawn_time = Instant::now();
        println!(
            "[spike] Container ready. Will attempt SetParent in 2s.\n\
             Make sure at least one VS Code window is already open."
        );
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.detach_on_exit();
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                self.layout_child();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.try_attach();
        if let Some(w) = self.window.as_ref() {
            w.request_redraw();
        }
    }
}

fn find_vscode_window() -> Option<HWND> {
    struct State {
        found: Option<HWND>,
    }
    let mut state = State { found: None };

    unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let state = &mut *(lparam.0 as *mut State);
        if !IsWindowVisible(hwnd).as_bool() {
            return TRUE;
        }
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return TRUE;
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, &mut buf);
        if copied <= 0 {
            return TRUE;
        }
        let title = String::from_utf16_lossy(&buf[..copied as usize]);
        if title.ends_with(VSCODE_TITLE_SUFFIX) {
            println!("[spike] candidate: {title}");
            state.found = Some(hwnd);
            return BOOL(0);
        }
        TRUE
    }

    unsafe {
        let _ = EnumWindows(
            Some(cb),
            LPARAM(&mut state as *mut _ as isize),
        );
    }
    state.found
}

// PWSTR is unused but kept for future expansion (creating Win32 controls
// inside the container needs it). Suppress unused warning.
#[allow(dead_code)]
fn _pwstr_dummy() -> PWSTR {
    PWSTR::null()
}

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("run_app");
}
