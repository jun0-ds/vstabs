// borderless-spike — vstabs feasibility check #3
//
// Sibling-slave + strip the child's WS_CAPTION / WS_THICKFRAME bits.
// Reparent is NOT used — the child remains top-level, so IME / focus / lifetime
// stay native, but it loses its OS frame and visually nests inside the
// container. Closest reachable approximation of "wrapping browser" UX without
// touching VS Code's parent.
//
// Restores original style on close so the user is not left with an
// unmovable borderless VS Code.

use std::collections::HashSet;
use std::process::Command;
use std::time::{Duration, Instant};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT, TRUE};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetClientRect, GetWindowLongPtrW, GetWindowRect,
    GetWindowTextLengthW, GetWindowTextW, IsWindowVisible, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, GWL_STYLE, HWND_TOP, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SW_RESTORE, WS_CAPTION, WS_THICKFRAME,
};

const TARGET_PATH: &str = r"C:\Temp";
const TAB_BAR_HEIGHT: i32 = 30;
const VSCODE_TITLE_SUFFIX: &str = " - Visual Studio Code";

struct App {
    window: Option<Window>,
    pre_spawn_hwnds: HashSet<isize>,
    spawn_started: Option<Instant>,
    spawn_attempted: bool,
    child_hwnd: Option<HWND>,
    original_style: Option<isize>,
    layout_calls: u64,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            pre_spawn_hwnds: HashSet::new(),
            spawn_started: None,
            spawn_attempted: false,
            child_hwnd: None,
            original_style: None,
            layout_calls: 0,
        }
    }

    fn spawn_vscode(&mut self) {
        if self.spawn_attempted {
            return;
        }
        self.spawn_attempted = true;
        self.pre_spawn_hwnds = enumerate_vscode_hwnds()
            .into_iter()
            .map(|h| h.0 as isize)
            .collect();
        println!(
            "[spike] {} pre-existing VS Code windows snapshotted (ignored).",
            self.pre_spawn_hwnds.len()
        );
        let result = Command::new("cmd")
            .args(["/C", "code", "--new-window", TARGET_PATH])
            .spawn();
        match result {
            Ok(child) => {
                println!(
                    "[spike] launched `code --new-window {}` (cmd PID {}).",
                    TARGET_PATH,
                    child.id()
                );
                self.spawn_started = Some(Instant::now());
            }
            Err(e) => {
                eprintln!("[spike] spawn failed: {e}. Is `code` on PATH?");
            }
        }
    }

    fn poll_for_child(&mut self) {
        if self.child_hwnd.is_some() {
            return;
        }
        let Some(start) = self.spawn_started else {
            return;
        };
        if start.elapsed() > Duration::from_secs(20) {
            eprintln!("[spike] timeout (20s) waiting for new VS Code window.");
            self.spawn_started = None;
            return;
        }
        for hwnd in enumerate_vscode_hwnds() {
            if !self.pre_spawn_hwnds.contains(&(hwnd.0 as isize)) {
                let title = hwnd_title(hwnd);
                let class = hwnd_class(hwnd);
                println!(
                    "[spike] captured hwnd={:?} after {:.1}s\n        title={:?}\n        class={:?}",
                    hwnd,
                    start.elapsed().as_secs_f32(),
                    title,
                    class
                );
                unsafe {
                    let original = GetWindowLongPtrW(hwnd, GWL_STYLE);
                    self.original_style = Some(original);
                    println!("[spike] original style=0x{:x}", original);

                    // Strip OS frame bits. Reparent is NOT done.
                    let strip_mask = (WS_CAPTION.0 | WS_THICKFRAME.0) as isize;
                    let new_style = original & !strip_mask;
                    SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);
                    println!("[spike] stripped style -> 0x{:x}", new_style);

                    // Unmaximize so subsequent SetWindowPos takes effect.
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                }
                self.child_hwnd = Some(hwnd);
                self.layout_child();
                return;
            }
        }
    }

    fn layout_child(&mut self) {
        let (Some(win), Some(child)) = (self.window.as_ref(), self.child_hwnd) else {
            return;
        };
        let container_hwnd = window_hwnd(win);
        unsafe {
            let mut rect = RECT::default();
            if let Err(e) = GetClientRect(container_hwnd, &mut rect) {
                eprintln!("[spike] GetClientRect failed: {e:?}");
                return;
            }
            let mut origin = POINT {
                x: 0,
                y: TAB_BAR_HEIGHT,
            };
            if !ClientToScreen(container_hwnd, &mut origin).as_bool() {
                eprintln!("[spike] ClientToScreen returned FALSE");
                return;
            }
            let w = rect.right - rect.left;
            let h = (rect.bottom - rect.top - TAB_BAR_HEIGHT).max(1);
            let result = SetWindowPos(
                child,
                HWND_TOP,
                origin.x,
                origin.y,
                w,
                h,
                SWP_NOACTIVATE | SWP_NOZORDER | SWP_FRAMECHANGED,
            );
            self.layout_calls += 1;
            if self.layout_calls <= 5 || self.layout_calls % 40 == 0 {
                let mut actual = RECT::default();
                let _ = GetWindowRect(child, &mut actual);
                println!(
                    "[spike] layout #{}: req=({},{}) {}x{}, actual=({},{}) {}x{}, swp={:?}",
                    self.layout_calls,
                    origin.x, origin.y, w, h,
                    actual.left, actual.top,
                    actual.right - actual.left,
                    actual.bottom - actual.top,
                    result
                );
            }
        }
    }

    fn restore_child_frame(&mut self) {
        let (Some(child), Some(orig)) = (self.child_hwnd, self.original_style) else {
            return;
        };
        unsafe {
            SetWindowLongPtrW(child, GWL_STYLE, orig);
            let _ = SetWindowPos(
                child,
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER | SWP_FRAMECHANGED,
            );
        }
        println!("[spike] restored child VS Code frame (style=0x{:x})", orig);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("vstabs borderless spike — close to restore VS Code frame")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0));
        let window = event_loop
            .create_window(attrs)
            .expect("create window failed");
        self.window = Some(window);
        println!("[spike] container ready. spawning VS Code at {} ...", TARGET_PATH);
        self.spawn_vscode();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.restore_child_frame();
                println!("[spike] container closing. Child VS Code stays alive with restored frame.");
                event_loop.exit();
            }
            WindowEvent::Resized(_) | WindowEvent::Moved(_) => {
                self.layout_child();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.poll_for_child();
        self.layout_child();
        std::thread::sleep(Duration::from_millis(50));
    }
}

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

fn hwnd_class(hwnd: HWND) -> String {
    unsafe {
        let mut buf = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut buf);
        if len <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

fn hwnd_title(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, &mut buf);
        if copied <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..copied as usize])
    }
}

fn enumerate_vscode_hwnds() -> Vec<HWND> {
    struct State {
        hwnds: Vec<HWND>,
    }
    let mut state = State { hwnds: Vec::new() };

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
            state.hwnds.push(hwnd);
        }
        TRUE
    }

    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut state as *mut _ as isize));
    }
    state.hwnds
}

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("run_app");
}
