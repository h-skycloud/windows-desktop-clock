use crate::{Clock, refresh, reposition, settings::FaceName, timers::*};
use std::ptr::{null, null_mut};
use windows_sys::{
    Win32::{
        Foundation::{FreeLibrary, HWND, LPARAM, SYSTEMTIME, WPARAM},
        Graphics::Gdi::LOGFONTW,
        System::{
            LibraryLoader::{
                GetModuleHandleW, GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
            },
            SystemInformation::{GetLocalTime, GetTickCount64},
        },
        UI::{
            Controls::{Dialogs::*, EM_SETLIMITTEXT},
            WindowsAndMessaging::*,
        },
    },
    core::PCWSTR,
    w,
};

pub const WATCH_TOGGLE: usize = 100;
pub const COUNT_TOGGLE: usize = 110;
const ADD_ALARM: usize = 120;
const TOGGLE_ALARM: usize = 130;
const REMOVE_ALARM: usize = 150;
const EDIT: i32 = 101;
const ERROR: i32 = 102;

pub fn append(root: HMENU, clock: &Clock) {
    let settings = clock.settings.get().timers;
    for (id, title, running, visible) in [
        (
            WATCH_TOGGLE,
            w!("Stopwatch"),
            clock.stopwatch.get().running(),
            settings.stopwatch.visible,
        ),
        (
            COUNT_TOGGLE,
            w!("Countdown"),
            clock.countdown.get().running(),
            settings.countdown.visible,
        ),
    ] {
        submenu(root, title, |menu| {
            item(
                menu,
                id,
                if running {
                    w!("Pause")
                } else {
                    w!("Start / Resume")
                },
            );
            item(menu, id + 1, w!("Reset"));
            if id == COUNT_TOGGLE {
                item(menu, id + 5, w!("Set duration…"));
            }
            unsafe {
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
            }
            item(menu, id + 2, w!("Change font…"));
            item(menu, id + 3, w!("Change color…"));
            unsafe {
                AppendMenuW(
                    menu,
                    MF_STRING | super::checked(visible),
                    id + 4,
                    w!("Show panel"),
                );
            }
        });
    }
    submenu(root, w!("Alarms"), |menu| {
        item(menu, ADD_ALARM, w!("Add daily alarm…"));
        if settings.alarms.iter().all(Option::is_none) {
            unsafe {
                AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, w!("No alarms set"));
            }
        }
        for (index, alarm) in settings.alarms.into_iter().enumerate() {
            if let Some(alarm) = alarm {
                let label = wide(&format!(
                    "{:02}:{:02} · {}",
                    alarm.minute / 60,
                    alarm.minute % 60,
                    if alarm.enabled { "On" } else { "Off" }
                ));
                submenu(menu, label.as_ptr(), |entry| {
                    item(
                        entry,
                        TOGGLE_ALARM + index,
                        if alarm.enabled {
                            w!("Disable")
                        } else {
                            w!("Enable")
                        },
                    );
                    item(entry, REMOVE_ALARM + index, w!("Remove"));
                });
            }
        }
    });
}

fn submenu(root: HMENU, label: PCWSTR, populate: impl FnOnce(HMENU)) {
    unsafe {
        let menu = CreatePopupMenu();
        if !menu.is_null() {
            populate(menu);
            if AppendMenuW(root, MF_POPUP, menu as usize, label) == 0 {
                DestroyMenu(menu);
            }
        }
    }
}

fn item(menu: HMENU, id: usize, label: PCWSTR) {
    unsafe {
        AppendMenuW(menu, MF_STRING, id, label);
    }
}

pub fn command(window: HWND, clock: &Clock, command: usize) -> bool {
    let mut settings = clock.settings.get();
    let previous = settings;
    let state = &mut settings.timers;
    let mut reset_countdown = false;
    let mut changed_alarm = None;
    let now = unsafe { GetTickCount64() };
    match command {
        WATCH_TOGGLE => {
            let mut stopwatch = clock.stopwatch.get();
            stopwatch.toggle(now);
            clock.stopwatch.set(stopwatch);
        }
        101 => clock.stopwatch.set(Stopwatch::default()),
        COUNT_TOGGLE => {
            let mut timer = clock.countdown.get();
            timer.toggle(now, state.duration);
            clock.countdown.set(timer);
        }
        111 => clock.countdown.set(Countdown::default()),
        102..=104 | 112..=114 => {
            let (panel, action) = if command < COUNT_TOGGLE {
                (&mut state.stopwatch, command - WATCH_TOGGLE)
            } else {
                (&mut state.countdown, command - COUNT_TOGGLE)
            };
            match action {
                2 => {
                    if let Some(font) = choose_font(window, panel.font) {
                        panel.font = font;
                    }
                }
                3 => {
                    if let Some(color) = crate::options::pick_color(window, panel.color) {
                        panel.color = color;
                    }
                }
                _ => panel.visible = !panel.visible,
            }
        }
        115 => {
            let initial = String::from_utf16_lossy(&duration_label(u64::from(state.duration)));
            if let Some(duration) = input(window, Kind::Duration, &initial) {
                state.duration = duration;
                reset_countdown = true;
            }
        }
        ADD_ALARM => {
            let Some(slot) = state.alarms.iter().position(Option::is_none) else {
                warning(
                    window,
                    w!("Remove an existing alarm before adding another."),
                );
                return true;
            };
            let local = local_time();
            let initial = format!("{:02}:{:02}", local.wHour, local.wMinute);
            if let Some(minute) = input(window, Kind::Alarm, &initial) {
                let minute = minute as u16;
                if state
                    .alarms
                    .iter()
                    .flatten()
                    .any(|alarm| alarm.minute == minute)
                {
                    warning(window, w!("An alarm is already set for that time."));
                    return true;
                }
                state.alarms[slot] = Some(Alarm {
                    minute,
                    enabled: true,
                });
                changed_alarm = Some((slot, minute));
            }
        }
        id if (TOGGLE_ALARM..TOGGLE_ALARM + ALARM_LIMIT).contains(&id) => {
            let slot = id - TOGGLE_ALARM;
            if let Some(alarm) = &mut state.alarms[slot] {
                alarm.enabled = !alarm.enabled;
                changed_alarm = Some((slot, alarm.minute));
            }
        }
        id if (REMOVE_ALARM..REMOVE_ALARM + ALARM_LIMIT).contains(&id) => {
            state.alarms[id - REMOVE_ALARM] = None;
        }
        _ => return false,
    }
    if settings.timers != previous.timers {
        settings = crate::settings::Settings {
            timers: settings.timers,
            ..clock.settings.get()
        };
        if settings.save().is_err() {
            warning(window, w!("Could not save the timer settings."));
            return true;
        }
        clock.settings.set(settings);
        if settings.timers.stopwatch.visible != previous.timers.stopwatch.visible
            || settings.timers.countdown.visible != previous.timers.countdown.visible
        {
            reposition(window, clock);
        }
    }
    if reset_countdown {
        clock.countdown.set(Countdown::default());
    }
    if let Some((slot, minute)) = changed_alarm {
        clock
            .alarm_history
            .borrow_mut()
            .changed(slot, &local_time(), minute);
    }
    refresh(window, clock);
    true
}

fn local_time() -> SYSTEMTIME {
    let mut local = SYSTEMTIME::default();
    unsafe {
        GetLocalTime(&mut local);
    }
    local
}

fn warning(window: HWND, text: PCWSTR) {
    unsafe {
        MessageBoxW(window, text, w!("Desktop Clock"), MB_OK | MB_ICONWARNING);
    }
}

fn choose_font(window: HWND, current: FaceName) -> Option<FaceName> {
    let mut font = LOGFONTW {
        lfHeight: -27,
        lfFaceName: current.0,
        ..Default::default()
    };
    let mut dialog = CHOOSEFONTW {
        lStructSize: size_of::<CHOOSEFONTW>() as u32,
        hwndOwner: window,
        lpLogFont: &mut font,
        Flags: CF_SCREENFONTS | CF_INITTOLOGFONTSTRUCT | CF_NOSIZESEL | CF_NOSTYLESEL,
        ..Default::default()
    };
    unsafe {
        let library = LoadLibraryExW(w!("comdlg32.dll"), null_mut(), LOAD_LIBRARY_SEARCH_SYSTEM32);
        if library.is_null() {
            return None;
        }
        let accepted =
            GetProcAddress(library, c"ChooseFontW".as_ptr().cast()).is_some_and(|entry| {
                let choose: unsafe extern "system" fn(*mut CHOOSEFONTW) -> i32 =
                    std::mem::transmute(entry);
                choose(&mut dialog) != 0
            });
        FreeLibrary(library);
        accepted.then_some(FaceName(font.lfFaceName))
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Duration,
    Alarm,
}

struct Input {
    kind: Kind,
    initial: Vec<u16>,
    result: Option<u32>,
}

fn input(owner: HWND, kind: Kind, initial: &str) -> Option<u32> {
    let (title, hint) = match kind {
        Kind::Duration => ("Set countdown duration", "Duration (HH:MM:SS)"),
        Kind::Alarm => ("Add daily alarm", "Local time (HH:MM, 24-hour)"),
    };
    let template = dialog_template(title, hint);
    let mut input = Input {
        kind,
        initial: wide(initial),
        result: None,
    };
    unsafe {
        DialogBoxIndirectParamW(
            GetModuleHandleW(null()),
            template.as_ptr().cast(),
            owner,
            Some(input_proc),
            (&raw mut input) as LPARAM,
        );
    }
    input.result
}

unsafe extern "system" fn input_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    unsafe {
        if message == WM_INITDIALOG {
            let input = &*(lparam as *const Input);
            SetWindowLongPtrW(window, GWLP_USERDATA, lparam);
            SetDlgItemTextW(window, EDIT, input.initial.as_ptr());
            SendDlgItemMessageW(window, EDIT, EM_SETLIMITTEXT, 8, 0);
            return 1;
        }
        match (message, (wparam & 0xffff) as i32) {
            (WM_COMMAND, IDOK) => {
                let input = &mut *(GetWindowLongPtrW(window, GWLP_USERDATA) as *mut Input);
                let mut text = [0; 10];
                let len = GetDlgItemTextW(window, EDIT, text.as_mut_ptr(), text.len() as i32);
                let text = String::from_utf16_lossy(&text[..len as usize]);
                input.result = match input.kind {
                    Kind::Duration => parse_duration(&text),
                    Kind::Alarm => parse_alarm(&text).map(u32::from),
                };
                if input.result.is_some() {
                    EndDialog(window, 1);
                } else {
                    SetDlgItemTextW(
                        window,
                        ERROR,
                        match input.kind {
                            Kind::Duration => w!("Enter a duration from 00:00:01 to 99:59:59."),
                            Kind::Alarm => w!("Enter a time from 00:00 to 23:59."),
                        },
                    );
                }
                1
            }
            (WM_COMMAND, IDCANCEL) | (WM_CLOSE, _) => {
                EndDialog(window, 0);
                1
            }
            _ => 0,
        }
    }
}

fn dialog_template(title: &str, hint: &str) -> Vec<u32> {
    let mut words = Vec::new();
    push_style(
        &mut words,
        WS_POPUP
            | WS_CAPTION
            | WS_SYSMENU
            | DS_MODALFRAME as u32
            | DS_CENTER as u32
            | DS_SETFONT as u32,
    );
    words.extend([5, 0, 0, 206, 79, 0, 0]);
    words.extend(wide(title));
    words.push(9);
    words.extend(wide("Segoe UI"));
    for (style, bounds, id, class, label) in [
        (0, [9, 8, 185, 10], 103, 0x82, hint),
        (
            WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL as u32,
            [9, 21, 188, 14],
            EDIT,
            0x81,
            "",
        ),
        (0, [9, 40, 188, 10], ERROR, 0x82, ""),
        (
            WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
            [93, 56, 50, 14],
            IDOK,
            0x80,
            "OK",
        ),
        (WS_TABSTOP, [147, 56, 50, 14], IDCANCEL, 0x80, "Cancel"),
    ] {
        if !words.len().is_multiple_of(2) {
            words.push(0);
        }
        push_style(&mut words, WS_CHILD | WS_VISIBLE | style);
        words.extend(bounds);
        words.extend([id as u16, 0xffff, class]);
        words.extend(wide(label));
        words.push(0);
    }
    // DLGTEMPLATE and DLGITEMTEMPLATE require DWORD alignment.
    words
        .chunks(2)
        .map(|pair| u32::from(pair[0]) | (u32::from(*pair.get(1).unwrap_or(&0)) << 16))
        .collect()
}

fn push_style(words: &mut Vec<u16>, style: u32) {
    words.extend([style as u16, (style >> 16) as u16, 0, 0]);
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}
