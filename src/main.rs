#![windows_subsystem = "windows"]

mod displays;
mod options;
mod renderer;
mod settings;
mod startup;

use settings::{Position, Settings, TRANSPARENCY_LEVELS};
use std::{
    cell::Cell,
    ptr::{null, null_mut},
};
use windows_sys::{
    Win32::{
        Foundation::{
            CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, POINT, RECT,
            SIZE, SYSTEMTIME, WPARAM,
        },
        Graphics::Gdi::{
            GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY,
            MONITORINFO, MonitorFromPoint, MonitorFromWindow,
        },
        System::{
            LibraryLoader::GetModuleHandleW, SystemInformation::GetLocalTime,
            Threading::CreateMutexW,
        },
        UI::{HiDpi::*, WindowsAndMessaging::*},
    },
    core::PCWSTR,
    w,
};

const MARGIN: i32 = 24;
const CLOCK_TIMER: usize = 1;
const SECONDS_COMMAND: usize = 1;
const CLOSE_COMMAND: usize = 2;
const OPTIONS_COMMAND: usize = 3;
const TRANSPARENCY_COMMAND: usize = 10;
const POSITION_COMMAND: usize = 20;
const TRANSPARENCY_LABELS: [PCWSTR; 5] = [w!("0%"), w!("25%"), w!("50%"), w!("75%"), w!("100%")];
const POSITION_LABELS: [PCWSTR; 7] = [
    w!("Top center"),
    w!("Top left"),
    w!("Top right"),
    w!("Bottom center"),
    w!("Bottom left"),
    w!("Bottom right"),
    w!("Last position"),
];

struct Clock {
    settings: Cell<Settings>,
    follows_anchor: Cell<bool>,
    options: Cell<HWND>,
}

fn main() {
    unsafe {
        // A single instance also prevents concurrent writes to the settings file.
        let lock = CreateMutexW(null(), 0, w!("Local\\MasaustuSaati.HHMM"));
        let already_running = GetLastError() == ERROR_ALREADY_EXISTS;
        if !lock.is_null() {
            if !already_running {
                run();
            }
            CloseHandle(lock);
        }
    }
}

unsafe fn run() {
    let Some(_font) = renderer::ClockFont::load() else {
        return;
    };
    let settings = Settings::load();
    let clock = Clock {
        settings: Cell::new(settings),
        follows_anchor: Cell::new(settings.position != Position::Last),
        options: Cell::new(null_mut()),
    };
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let instance = GetModuleHandleW(null());
        let class_name = w!("MasaustuSaatiWindow");
        let window_class = WNDCLASSW {
            hInstance: instance,
            lpszClassName: class_name,
            hCursor: LoadCursorW(null_mut(), IDC_SIZEALL),
            lpfnWndProc: Some(window_proc),
            ..Default::default()
        };
        if RegisterClassW(&window_class) == 0 {
            return;
        }

        let dpi = GetDpiForSystem();
        let size = clock_size(settings, dpi);
        let origin = startup_origin(settings, size, dpi);
        let window = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED,
            class_name,
            w!("Desktop Clock"),
            WS_POPUP,
            origin.x,
            origin.y,
            size.cx,
            size.cy,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );
        if window.is_null() {
            return;
        }
        // The state lives on this stack until the window and its message loop are gone.
        SetWindowLongPtrW(window, GWLP_USERDATA, (&raw const clock) as isize);
        reposition(window, &clock);
        if !refresh(window, settings) {
            DestroyWindow(window);
            return;
        }
        lower(window, SWP_SHOWWINDOW);

        let mut message = MSG::default();
        while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
            let options = clock.options.get();
            if !options.is_null() && IsDialogMessageW(options, &message) != 0 {
                continue;
            }
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        if IsWindow(window) != 0 {
            DestroyWindow(window);
        }
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        let state = GetWindowLongPtrW(window, GWLP_USERDATA) as *const Clock;
        let Some(clock) = state.as_ref() else {
            return DefWindowProcW(window, message, wparam, lparam);
        };
        let settings = clock.settings.get();
        match message {
            WM_TIMER if wparam == CLOCK_TIMER => {
                refresh(window, settings);
            }
            WM_TIMECHANGE => {
                refresh(window, settings);
            }
            WM_POWERBROADCAST if wparam == PBT_APMRESUMEAUTOMATIC as usize => {
                refresh(window, settings);
                return 1;
            }
            WM_DISPLAYCHANGE | WM_SETTINGCHANGE => {
                reposition(window, clock);
                refresh(window, settings);
            }
            WM_ENTERSIZEMOVE => {
                clock.follows_anchor.set(false);
            }
            WM_EXITSIZEMOVE => {
                fit_to_monitor(window, settings);
                if matches!(settings.position, Position::Last | Position::Custom) {
                    save_settings(window, clock, settings);
                }
                lower(window, 0);
            }
            WM_DPICHANGED => {
                let suggested = &*(lparam as *const RECT);
                let dpi = (wparam & 0xffff) as u32;
                let size = clock_size(settings, dpi);
                place_window(
                    window,
                    if clock.follows_anchor.get() {
                        startup_origin(settings, size, dpi)
                    } else {
                        POINT {
                            x: suggested.left,
                            y: suggested.top,
                        }
                    },
                    size,
                );
                refresh(window, settings);
            }
            WM_NCHITTEST => return HTCAPTION as LRESULT,
            WM_NCRBUTTONUP | WM_CONTEXTMENU => show_menu(window, settings),
            WM_COMMAND if lparam == 0 && (wparam >> 16) == 0 => {
                apply_menu_command(window, clock, wparam);
            }
            WM_DESTROY => {
                KillTimer(window, CLOCK_TIMER);
                PostQuitMessage(0);
            }
            _ => return DefWindowProcW(window, message, wparam, lparam),
        }
    }
    0
}

fn scale(value: i32, dpi: u32) -> i32 {
    (value * dpi as i32 + 48) / 96
}

fn clock_size(settings: Settings, dpi: u32) -> SIZE {
    renderer::size(settings, dpi)
}

fn refresh(window: HWND, settings: Settings) -> bool {
    let mut now = SYSTEMTIME::default();
    unsafe { GetLocalTime(&mut now) };
    let period = if settings.seconds { 1_000 } else { 60_000 };
    let elapsed = u32::from(now.wSecond) * 1_000 + u32::from(now.wMilliseconds);
    let delay = period - elapsed % period + 12;
    let tolerance = if settings.seconds { 25 } else { 100 };
    let timer = unsafe { SetCoalescableTimer(window, CLOCK_TIMER, delay, None, tolerance) };

    let digit = |value: u16| u16::from(b'0') + value;
    let label = [
        digit(now.wHour / 10),
        digit(now.wHour % 10),
        u16::from(b':'),
        digit(now.wMinute / 10),
        digit(now.wMinute % 10),
        u16::from(b':'),
        digit(now.wSecond / 10),
        digit(now.wSecond % 10),
    ];
    let dpi = unsafe { GetDpiForWindow(window) };
    renderer::present(
        window,
        dpi,
        &label[..if settings.seconds { 8 } else { 5 }],
        settings,
    ) && timer != 0
}

fn startup_origin(settings: Settings, size: SIZE, dpi: u32) -> POINT {
    if settings.position == Position::Custom
        && let Some(display) =
            displays::selected(&displays::connected(), settings.placement.monitor)
    {
        return displays::place(display.work, size, settings.placement, dpi);
    }
    if settings.position == Position::Last
        && let Some((x, y)) = settings.last_origin
    {
        let saved = POINT { x, y };
        let monitor = unsafe { MonitorFromPoint(saved, MONITOR_DEFAULTTONEAREST) };
        return constrain(saved, size, work_area(monitor));
    }
    let primary = unsafe { MonitorFromPoint(POINT::default(), MONITOR_DEFAULTTOPRIMARY) };
    let area = work_area(primary);
    let margin = scale(MARGIN, dpi);
    let x = match settings.position {
        Position::TopLeft | Position::BottomLeft => area.left + margin,
        Position::TopRight | Position::BottomRight => area.right - size.cx - margin,
        _ => area.left + (area.right - area.left - size.cx) / 2,
    };
    let y = match settings.position {
        Position::BottomCenter | Position::BottomLeft | Position::BottomRight => {
            area.bottom - size.cy - margin
        }
        _ => area.top + margin,
    };
    constrain(POINT { x, y }, size, area)
}

fn work_area(monitor: HMONITOR) -> RECT {
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        if GetMonitorInfoW(monitor, &mut info) != 0 {
            return info.rcWork;
        }
        RECT {
            right: GetSystemMetrics(SM_CXSCREEN),
            bottom: GetSystemMetrics(SM_CYSCREEN),
            ..Default::default()
        }
    }
}

fn constrain(origin: POINT, size: SIZE, area: RECT) -> POINT {
    POINT {
        x: origin
            .x
            .clamp(area.left, (area.right - size.cx).max(area.left)),
        y: origin
            .y
            .clamp(area.top, (area.bottom - size.cy).max(area.top)),
    }
}

fn window_bounds(window: HWND) -> Option<RECT> {
    let mut bounds = RECT::default();
    (unsafe { GetWindowRect(window, &mut bounds) } != 0).then_some(bounds)
}

fn fit_to_monitor(window: HWND, settings: Settings) {
    let Some(bounds) = window_bounds(window) else {
        return;
    };
    let dpi = unsafe { GetDpiForWindow(window) };
    let size = clock_size(settings, dpi);
    let width = bounds.right - bounds.left;
    let height = bounds.bottom - bounds.top;
    let center_aligned = POINT {
        x: bounds.left + (width - size.cx) / 2,
        y: bounds.top,
    };
    let monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };
    let origin = constrain(center_aligned, size, work_area(monitor));
    if (origin.x, origin.y, size.cx, size.cy) != (bounds.left, bounds.top, width, height) {
        place_window(window, origin, size);
    }
}

fn reposition(window: HWND, clock: &Clock) {
    let settings = clock.settings.get();
    if clock.follows_anchor.get() {
        let dpi = unsafe { GetDpiForWindow(window) };
        let size = clock_size(settings, dpi);
        place_window(window, startup_origin(settings, size, dpi), size);
    } else {
        fit_to_monitor(window, settings);
    }
}

fn place_window(window: HWND, origin: POINT, size: SIZE) {
    unsafe {
        SetWindowPos(
            window,
            null_mut(),
            origin.x,
            origin.y,
            size.cx,
            size.cy,
            SWP_NOACTIVATE | SWP_NOZORDER,
        );
    }
}

fn lower(window: HWND, flags: u32) {
    unsafe {
        SetWindowPos(
            window,
            HWND_BOTTOM,
            0,
            0,
            0,
            0,
            flags | SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

fn save_settings(window: HWND, clock: &Clock, mut settings: Settings) {
    if settings.position == Position::Last
        && let Some(bounds) = window_bounds(window)
    {
        settings.last_origin = Some((bounds.left, bounds.top));
    }
    if settings.position == Position::Custom && !clock.follows_anchor.get() {
        record_placement(window, &mut settings);
    }
    clock.settings.set(settings);
    if settings.save().is_err() {
        unsafe {
            MessageBoxW(
                window,
                w!("Could not save settings."),
                w!("Desktop Clock"),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
}

fn record_placement(window: HWND, settings: &mut Settings) {
    let Some(bounds) = window_bounds(window) else {
        return;
    };
    let size = SIZE {
        cx: bounds.right - bounds.left,
        cy: bounds.bottom - bounds.top,
    };
    let center = POINT {
        x: bounds.left + size.cx / 2,
        y: bounds.top + size.cy / 2,
    };
    let screens = displays::connected();
    if let Some(display) = screens
        .iter()
        .find(|display| displays::contains(display.area, center))
    {
        let placement = &mut settings.placement;
        placement.monitor = display.id;
        let dpi = unsafe { GetDpiForWindow(window) };
        (placement.x, placement.y) = displays::between(
            display.work,
            size,
            *placement,
            POINT {
                x: bounds.left,
                y: bounds.top,
            },
            dpi,
        );
    }
}

fn close_options(window: HWND, dialog: HWND) {
    unsafe {
        if let Some(clock) = (GetWindowLongPtrW(window, GWLP_USERDATA) as *const Clock).as_ref()
            && clock.options.get() == dialog
        {
            clock.options.set(null_mut());
        }
    }
}

fn apply_options(
    window: HWND,
    settings: Settings,
    change_startup: Option<bool>,
) -> std::io::Result<()> {
    let clock = unsafe { (GetWindowLongPtrW(window, GWLP_USERDATA) as *const Clock).as_ref() };
    let Some(clock) = clock else {
        return Err(std::io::ErrorKind::NotFound.into());
    };
    let old = clock.settings.get();
    if let Some(enable) = change_startup {
        startup::set(enable)?;
    }
    if settings != old
        && let Err(error) = settings.save()
    {
        if let Some(enable) = change_startup {
            let _ = startup::set(!enable);
        }
        return Err(error);
    }
    clock.settings.set(settings);
    if settings.position != old.position
        || (settings.position == Position::Custom && settings.placement != old.placement)
    {
        clock
            .follows_anchor
            .set(settings.position != Position::Last);
    }
    reposition(window, clock);
    refresh(window, settings);
    Ok(())
}

fn apply_menu_command(window: HWND, clock: &Clock, command: usize) {
    let mut settings = clock.settings.get();
    match command {
        CLOSE_COMMAND => {
            unsafe { DestroyWindow(window) };
            return;
        }
        OPTIONS_COMMAND => {
            let current = clock.options.get();
            if current.is_null() {
                clock.options.set(options::open(window, settings));
            } else {
                unsafe { SetForegroundWindow(current) };
            }
            return;
        }
        SECONDS_COMMAND => {
            settings.seconds = !settings.seconds;
            clock.settings.set(settings);
            reposition(window, clock);
        }
        id if (TRANSPARENCY_COMMAND..TRANSPARENCY_COMMAND + TRANSPARENCY_LEVELS.len())
            .contains(&id) =>
        {
            settings.transparency = TRANSPARENCY_LEVELS[id - TRANSPARENCY_COMMAND];
        }
        id if (POSITION_COMMAND..POSITION_COMMAND + Position::ALL.len()).contains(&id) => {
            settings.position = Position::ALL[id - POSITION_COMMAND];
            clock
                .follows_anchor
                .set(settings.position != Position::Last);
            clock.settings.set(settings);
            reposition(window, clock);
        }
        _ => return,
    }
    save_settings(window, clock, settings);
    refresh(window, settings);
}

fn checked(selected: bool) -> u32 {
    if selected { MF_CHECKED } else { MF_UNCHECKED }
}

fn show_menu(window: HWND, settings: Settings) {
    unsafe {
        let mut cursor = POINT::default();
        if GetCursorPos(&mut cursor) == 0 {
            return;
        }
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        let transparency = CreatePopupMenu();
        if !transparency.is_null() {
            for (index, level) in TRANSPARENCY_LEVELS.into_iter().enumerate() {
                AppendMenuW(
                    transparency,
                    MF_STRING | checked(level == settings.transparency),
                    TRANSPARENCY_COMMAND + index,
                    TRANSPARENCY_LABELS[index],
                );
            }
            if AppendMenuW(
                menu,
                MF_POPUP,
                transparency as usize,
                w!("Background transparency"),
            ) == 0
            {
                DestroyMenu(transparency);
            }
        }
        AppendMenuW(
            menu,
            MF_STRING | checked(settings.seconds),
            SECONDS_COMMAND,
            w!("Show seconds"),
        );
        let positions = CreatePopupMenu();
        if !positions.is_null() {
            for (index, position) in Position::ALL.into_iter().enumerate() {
                AppendMenuW(
                    positions,
                    MF_STRING | checked(position == settings.position),
                    POSITION_COMMAND + index,
                    POSITION_LABELS[index],
                );
            }
            if AppendMenuW(menu, MF_POPUP, positions as usize, w!("Startup position")) == 0 {
                DestroyMenu(positions);
            }
        }
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        AppendMenuW(menu, MF_STRING, OPTIONS_COMMAND, w!("Options…"));
        AppendMenuW(menu, MF_STRING, CLOSE_COMMAND, w!("Close"));
        SetForegroundWindow(window);
        TrackPopupMenu(menu, TPM_RIGHTBUTTON, cursor.x, cursor.y, 0, window, null());
        PostMessageW(window, WM_NULL, 0, 0);
        DestroyMenu(menu);
        lower(window, 0);
    }
}
