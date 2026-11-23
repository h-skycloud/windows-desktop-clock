mod timer_page;
mod view;
use timer_page::*;

use crate::{
    displays::{self, Display, Map},
    renderer, scale,
    settings::{FaceName, MAX_DIGIT_SIZE, MIN_DIGIT_SIZE, Position, Settings},
    startup,
    timers::TimerSettings,
};
use std::{
    cell::{Cell, RefCell},
    ptr::{null, null_mut},
};
use windows_sys::{
    Win32::{
        Foundation::{FreeLibrary, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::Gdi::*,
        System::LibraryLoader::{
            GetModuleHandleW, GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
        },
        UI::{
            Controls::{DRAWITEMSTRUCT, Dialogs::*, EM_SETLIMITTEXT},
            HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow},
            Input::KeyboardAndMouse::{
                EnableWindow, GetFocus, ReleaseCapture, SetCapture, SetFocus,
            },
            WindowsAndMessaging::*,
        },
    },
    core::PCWSTR,
    w,
};

const WIDTH: i32 = 900;
const HEIGHT: i32 = 585;
const NAV: usize = 100;
const STARTUP: usize = 110;
const SECONDS: usize = 111;
const FONT: usize = 120;
const SIZE_H: usize = 121;
const SIZE_M: usize = 122;
const SIZE_S: usize = 123;
const TRANSPARENCY: usize = 124;
const INK: usize = 125;
const BACKGROUND: usize = 126;
const SCREEN: usize = 130;
const DOCK: usize = 131;
const INSET: usize = 132;
const ANGLE: usize = 133;
const ANGLE_45: usize = 134;
const APPLY: usize = 1;
const CANCEL: usize = 2;
const COMBO_PROC: PCWSTR = w!("MasaustuSaati.NativeComboProc");

const TABS: [PCWSTR; 4] = [
    w!("General"),
    w!("Appearance"),
    w!("Position"),
    w!("Timers"),
];
const DOCKS: [(PCWSTR, u16, u16); 9] = [
    (w!("Top left"), 0, 0),
    (w!("Top center"), 500, 0),
    (w!("Top right"), 1000, 0),
    (w!("Left edge"), 0, 500),
    (w!("Center"), 500, 500),
    (w!("Right edge"), 1000, 500),
    (w!("Bottom left"), 0, 1000),
    (w!("Bottom center"), 500, 1000),
    (w!("Bottom right"), 1000, 1000),
];

struct Child {
    handle: HWND,
    id: usize,
    page: Option<usize>,
    rect: RECT,
}

struct Options {
    clock: HWND,
    draft: Cell<Settings>,
    original_timers: TimerSettings,
    page: Cell<usize>,
    loading: Cell<bool>,
    startup: Cell<Option<bool>>,
    original_startup: Option<bool>,
    density: Cell<u32>,
    fonts: RefCell<view::Fonts>,
    faces: Vec<FaceName>,
    screens: RefCell<Vec<Display>>,
    controls: RefCell<Vec<Child>>,
    dragging: Cell<bool>,
}

pub fn open(clock: HWND, mut draft: Settings) -> HWND {
    let screens = displays::connected();
    if draft.position != Position::Custom {
        super::record_placement(clock, &mut draft);
    }
    let startup = startup::enabled().ok();
    let faces = installed_faces(draft.appearance.font);
    unsafe {
        let instance = GetModuleHandleW(null());
        let name = w!("MasaustuSaatiOptions");
        let class = WNDCLASSW {
            hInstance: instance,
            lpszClassName: name,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            lpfnWndProc: Some(proc),
            ..Default::default()
        };
        RegisterClassW(&class);
        let monitor = MonitorFromWindow(clock, MONITOR_DEFAULTTONEAREST);
        let area = super::work_area(monitor);
        let density = fit_density(GetDpiForWindow(clock), area);
        let style = WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN;
        let mut outer = view::rect(0, 0, scale(WIDTH, density), scale(HEIGHT, density));
        AdjustWindowRectExForDpi(
            &mut outer,
            style,
            0,
            WS_EX_APPWINDOW,
            GetDpiForWindow(clock),
        );
        let width = outer.right - outer.left;
        let height = outer.bottom - outer.top;
        let window = CreateWindowExW(
            WS_EX_APPWINDOW,
            name,
            w!("Desktop Clock — Options"),
            style,
            area.left + (area.right - area.left - width) / 2,
            area.top + (area.bottom - area.top - height) / 2,
            width,
            height,
            clock,
            null_mut(),
            instance,
            null(),
        );
        if window.is_null() {
            return window;
        }
        let options = Box::new(Options {
            clock,
            draft: Cell::new(draft),
            original_timers: draft.timers,
            page: Cell::new(0),
            loading: Cell::new(false),
            startup: Cell::new(startup),
            original_startup: startup,
            density: Cell::new(density),
            fonts: RefCell::new(view::Fonts::new(density)),
            faces,
            screens: RefCell::new(screens),
            controls: RefCell::new(Vec::new()),
            dragging: Cell::new(false),
        });
        let raw = Box::into_raw(options);
        SetWindowLongPtrW(window, GWLP_USERDATA, raw as isize);
        (*raw).create_controls(window);
        (*raw).sync_values();
        (*raw).change_page(window, 0, false);
        ShowWindow(window, SW_SHOWNORMAL);
        SetForegroundWindow(window);
        SetFocus((*raw).control(NAV));
        window
    }
}

fn fit_density(dpi: u32, area: RECT) -> u32 {
    let width = (area.right - area.left - 32).max(1) as u32;
    let height = (area.bottom - area.top - 86).max(1) as u32;
    dpi.min(width * 96 / WIDTH as u32)
        .min(height * 96 / HEIGHT as u32)
        .max(64)
}

unsafe extern "system" fn proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        let raw = GetWindowLongPtrW(window, GWLP_USERDATA) as *mut Options;
        let Some(options) = raw.as_ref() else {
            return DefWindowProcW(window, message, wparam, lparam);
        };
        match message {
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let dc = BeginPaint(window, &mut ps);
                options.paint(dc, ps.rcPaint);
                EndPaint(window, &ps);
            }
            WM_PRINTCLIENT => {
                let mut area = RECT::default();
                GetClientRect(window, &mut area);
                options.paint(wparam as HDC, area);
            }
            WM_DRAWITEM => {
                options.paint_button(&*(lparam as *const DRAWITEMSTRUCT));
                return 1;
            }
            WM_COMMAND if !options.loading.get() => {
                options.command(window, wparam & 0xffff, (wparam >> 16) as u32);
            }
            WM_LBUTTONDOWN if options.page.get() == 2 => {
                if options.map_select(window, point(lparam)) {
                    options.dragging.set(true);
                    SetCapture(window);
                }
            }
            WM_MOUSEMOVE if options.dragging.get() => {
                options.map_select(window, point(lparam));
            }
            WM_LBUTTONUP if options.dragging.replace(false) => {
                options.map_select(window, point(lparam));
                ReleaseCapture();
            }
            WM_CAPTURECHANGED => options.dragging.set(false),
            WM_DISPLAYCHANGE => {
                options.screens.replace(displays::connected());
                options.sync_screens();
                InvalidateRect(window, null(), 0);
            }
            WM_DPICHANGED => {
                let offered = &*(lparam as *const RECT);
                let monitor = MonitorFromRect(offered, MONITOR_DEFAULTTONEAREST);
                let dpi = (wparam & 0xffff) as u32;
                let density = fit_density(dpi, super::work_area(monitor));
                options.density.set(density);
                let old_fonts = options.fonts.replace(view::Fonts::new(density));
                options.layout_controls();
                drop(old_fonts);
                let mut outer = view::rect(0, 0, scale(WIDTH, density), scale(HEIGHT, density));
                AdjustWindowRectExForDpi(
                    &mut outer,
                    WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN,
                    0,
                    WS_EX_APPWINDOW,
                    dpi,
                );
                SetWindowPos(
                    window,
                    null_mut(),
                    offered.left,
                    offered.top,
                    outer.right - outer.left,
                    outer.bottom - outer.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                InvalidateRect(window, null(), 0);
            }
            WM_CLOSE => {
                DestroyWindow(window);
            }
            WM_NCDESTROY => {
                super::close_options(options.clock, window);
                SetWindowLongPtrW(window, GWLP_USERDATA, 0);
                drop(Box::from_raw(raw));
                return DefWindowProcW(window, message, wparam, lparam);
            }
            _ => return DefWindowProcW(window, message, wparam, lparam),
        }
    }
    0
}

impl Options {
    fn create_controls(&self, window: HWND) {
        let button = w!("BUTTON");
        let edit = w!("EDIT");
        let combo = w!("COMBOBOX");
        let button_style = BS_OWNERDRAW as u32;
        let edit_style = ES_AUTOHSCROLL as u32;
        let combo_style = CBS_DROPDOWNLIST as u32 | WS_VSCROLL;
        for (index, label) in TABS.into_iter().enumerate() {
            self.add(
                window,
                NAV + index,
                None,
                button,
                label,
                button_style,
                view::rect(14, 82 + index as i32 * 49, 154, 43),
            );
        }
        self.add(
            window,
            STARTUP,
            Some(0),
            button,
            w!("Start with Windows"),
            button_style,
            view::rect(229, 181, 624, 64),
        );
        self.add(
            window,
            SECONDS,
            Some(0),
            button,
            w!("Show seconds"),
            button_style,
            view::rect(229, 255, 624, 64),
        );
        self.add(
            window,
            FONT,
            Some(1),
            combo,
            null(),
            combo_style,
            view::rect(232, 213, 272, 240),
        );
        for (index, id) in [SIZE_H, SIZE_M, SIZE_S].into_iter().enumerate() {
            self.add(
                window,
                id,
                Some(1),
                edit,
                null(),
                edit_style,
                view::rect(232 + index as i32 * 93, 288, 78, 32),
            );
        }
        self.add(
            window,
            TRANSPARENCY,
            Some(1),
            edit,
            null(),
            edit_style,
            view::rect(232, 356, 78, 32),
        );
        self.add(
            window,
            INK,
            Some(1),
            button,
            w!("Digit color"),
            button_style,
            view::rect(232, 429, 128, 34),
        );
        self.add(
            window,
            BACKGROUND,
            Some(1),
            button,
            w!("Background color"),
            button_style,
            view::rect(376, 429, 128, 34),
        );
        self.add(
            window,
            SCREEN,
            Some(2),
            combo,
            null(),
            combo_style,
            view::rect(232, 452, 162, 200),
        );
        self.add(
            window,
            DOCK,
            Some(2),
            combo,
            null(),
            combo_style,
            view::rect(408, 452, 136, 250),
        );
        self.add(
            window,
            INSET,
            Some(2),
            edit,
            null(),
            edit_style,
            view::rect(558, 452, 82, 32),
        );
        self.add(
            window,
            ANGLE,
            Some(2),
            edit,
            null(),
            edit_style,
            view::rect(654, 452, 76, 32),
        );
        self.add(
            window,
            ANGLE_45,
            Some(2),
            button,
            w!("Apply 45°"),
            button_style,
            view::rect(746, 452, 99, 32),
        );
        self.add(
            window,
            CANCEL,
            None,
            button,
            w!("Cancel"),
            button_style,
            view::rect(648, 540, 94, 34),
        );
        self.add(
            window,
            APPLY,
            None,
            button,
            w!("Save"),
            button_style,
            view::rect(754, 540, 114, 34),
        );
        self.create_timer_controls(window);
        self.layout_controls();
        unsafe {
            for face in &self.faces {
                SendMessageW(
                    self.control(FONT),
                    CB_ADDSTRING,
                    0,
                    face.0.as_ptr() as LPARAM,
                );
            }
            SendMessageW(
                self.control(DOCK),
                CB_ADDSTRING,
                0,
                w!("Free placement") as LPARAM,
            );
            for (label, _, _) in DOCKS {
                SendMessageW(self.control(DOCK), CB_ADDSTRING, 0, label as LPARAM);
            }
            EnableWindow(self.control(STARTUP), self.startup.get().is_some() as i32);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn add(
        &self,
        parent: HWND,
        id: usize,
        page: Option<usize>,
        class: PCWSTR,
        text: PCWSTR,
        style: u32,
        rect: RECT,
    ) {
        let handle = unsafe {
            CreateWindowExW(
                0,
                class,
                text,
                style | WS_CHILD | WS_TABSTOP,
                0,
                0,
                1,
                1,
                parent,
                id as HMENU,
                GetModuleHandleW(null()),
                null(),
            )
        };
        if matches!(id, FONT | SCREEN | DOCK | ALARM_LIST) && !handle.is_null() {
            unsafe {
                let original = GetWindowLongPtrW(handle, GWLP_WNDPROC);
                if original != 0 && SetPropW(handle, COMBO_PROC, original as _) != 0 {
                    SetWindowLongPtrW(handle, GWLP_WNDPROC, combo_proc as *const () as isize);
                }
            }
        }
        self.controls.borrow_mut().push(Child {
            handle,
            id,
            page,
            rect,
        });
    }

    fn layout_controls(&self) {
        let density = self.density.get();
        let font = self.fonts.borrow().body;
        for child in self.controls.borrow().iter() {
            let numeric = numeric_field(child.id);
            let mut bounds = view::scaled(child.rect, density);
            if numeric {
                let padding = if matches!(child.id, WATCH_TRANSPARENCY | COUNT_TRANSPARENCY) {
                    6
                } else {
                    10
                };
                bounds.left += scale(padding, density);
                bounds.right -= scale(padding, density);
                bounds.top += scale(7, density);
                bounds.bottom -= scale(6, density);
            }
            unsafe {
                SetWindowPos(
                    child.handle,
                    null_mut(),
                    bounds.left,
                    bounds.top,
                    bounds.right - bounds.left,
                    bounds.bottom - bounds.top,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                );
                SendMessageW(child.handle, WM_SETFONT, font as WPARAM, 1);
                if numeric {
                    SendMessageW(child.handle, EM_SETLIMITTEXT, 4, 0);
                }
                if matches!(child.id, FONT | SCREEN | DOCK | ALARM_LIST) {
                    SendMessageW(
                        child.handle,
                        CB_SETITEMHEIGHT,
                        usize::MAX,
                        scale(25, density) as LPARAM,
                    );
                }
            }
        }
    }

    fn control(&self, id: usize) -> HWND {
        self.controls
            .borrow()
            .iter()
            .find(|child| child.id == id)
            .map_or(null_mut(), |child| child.handle)
    }

    fn change_page(&self, window: HWND, page: usize, focus: bool) {
        self.page.set(page);
        for child in self.controls.borrow().iter() {
            let show = child.page.is_none_or(|child_page| child_page == page);
            unsafe {
                ShowWindow(child.handle, if show { SW_SHOWNA } else { SW_HIDE });
            }
        }
        if focus {
            unsafe {
                SetFocus(self.control(NAV + page));
            }
        }
        unsafe { InvalidateRect(window, null(), 0) };
    }

    fn sync_values(&self) {
        self.loading.set(true);
        let draft = self.draft.get();
        self.set_number(SIZE_H, draft.appearance.sizes[0].into());
        self.set_number(SIZE_M, draft.appearance.sizes[1].into());
        self.set_number(SIZE_S, draft.appearance.sizes[2].into());
        self.set_number(TRANSPARENCY, draft.transparency.into());
        self.set_number(
            WATCH_TRANSPARENCY,
            draft.timers.stopwatch.transparency.into(),
        );
        self.set_number(
            COUNT_TRANSPARENCY,
            draft.timers.countdown.transparency.into(),
        );
        self.set_number(INSET, draft.placement.inset.into());
        self.set_number(ANGLE, draft.placement.angle.into());
        self.set_combo(
            FONT,
            self.faces
                .iter()
                .position(|&face| face == draft.appearance.font)
                .unwrap_or(0),
        );
        self.sync_screens();
        self.sync_dock();
        self.sync_timer_controls(None);
        self.loading.set(false);
    }

    fn set_number(&self, id: usize, value: i32) {
        let value = wide(&value.to_string());
        unsafe {
            SetWindowTextW(self.control(id), value.as_ptr());
        }
    }

    fn set_combo(&self, id: usize, index: usize) {
        unsafe {
            SendMessageW(self.control(id), CB_SETCURSEL, index, 0);
        }
    }

    fn combo_index(&self, id: usize) -> usize {
        unsafe { SendMessageW(self.control(id), CB_GETCURSEL, 0, 0) as usize }
    }

    fn sync_screens(&self) {
        let control = self.control(SCREEN);
        let screens = self.screens.borrow();
        unsafe {
            SendMessageW(control, CB_RESETCONTENT, 0, 0);
            for screen in &*screens {
                let mut label = screen_name(screen);
                if screen.main {
                    label.push_str(" · Primary");
                }
                let label = wide(&label);
                SendMessageW(control, CB_ADDSTRING, 0, label.as_ptr() as LPARAM);
            }
        }
        self.set_combo(
            SCREEN,
            screens
                .iter()
                .position(|screen| screen.id == self.draft.get().placement.monitor)
                .unwrap_or(0),
        );
    }

    fn sync_dock(&self) {
        let draft = self.draft.get();
        let placement = draft.placement;
        let index = DOCKS
            .iter()
            .position(|&(_, x, y)| x == placement.x && y == placement.y)
            .map_or(0, |index| index + 1);
        self.set_combo(DOCK, index);
    }

    fn number(&self, id: usize) -> Option<i32> {
        let mut text = [0; 8];
        let count =
            unsafe { GetWindowTextW(self.control(id), text.as_mut_ptr(), text.len() as i32) };
        let value: i32 = String::from_utf16_lossy(&text[..count as usize])
            .parse()
            .ok()?;
        let (min, max) = match id {
            SIZE_H | SIZE_M | SIZE_S => (i32::from(MIN_DIGIT_SIZE), i32::from(MAX_DIGIT_SIZE)),
            TRANSPARENCY | WATCH_TRANSPARENCY | COUNT_TRANSPARENCY => (0, 100),
            INSET => (0, 120),
            ANGLE => (-180, 180),
            _ => return None,
        };
        (min..=max).contains(&value).then_some(value)
    }

    fn number_changed(&self, id: usize) {
        let Some(value) = self.number(id) else { return };
        let mut settings = self.draft.get();
        match id {
            SIZE_H | SIZE_M | SIZE_S => settings.appearance.sizes[id - SIZE_H] = value as u16,
            TRANSPARENCY => settings.transparency = value as u8,
            WATCH_TRANSPARENCY => settings.timers.stopwatch.transparency = value as u8,
            COUNT_TRANSPARENCY => settings.timers.countdown.transparency = value as u8,
            INSET => {
                settings.placement.inset = value as u16;
                settings.position = Position::Custom;
            }
            ANGLE => settings.placement.angle = value as i16,
            _ => return,
        }
        self.draft.set(settings);
    }

    fn command(&self, window: HWND, id: usize, event: u32) {
        if self.timer_command(window, id, event) {
            return;
        }
        if numeric_field(id) && matches!(event, EN_SETFOCUS | EN_KILLFOCUS) {
            if let Some(child) = self.controls.borrow().iter().find(|child| child.id == id) {
                let bounds = view::scaled(child.rect, self.density.get());
                unsafe { InvalidateRect(window, &bounds, 0) };
            }
            return;
        }
        let mut draft = self.draft.get();
        match (id, event) {
            (NAV..=103, BN_CLICKED) => {
                self.change_page(window, id - NAV, true);
                return;
            }
            (STARTUP, BN_CLICKED) => self.startup.set(self.startup.get().map(|value| !value)),
            (SECONDS, BN_CLICKED) => {
                draft.seconds = !draft.seconds;
                self.draft.set(draft);
            }
            (FONT, CBN_SELCHANGE) => {
                if let Some(&face) = self.faces.get(self.combo_index(FONT)) {
                    draft.appearance.font = face;
                    self.draft.set(draft);
                }
            }
            (SCREEN, CBN_SELCHANGE) => {
                if let Some(screen) = self.screens.borrow().get(self.combo_index(SCREEN)) {
                    draft.placement.monitor = screen.id;
                    draft.position = Position::Custom;
                    self.draft.set(draft);
                }
            }
            (DOCK, CBN_SELCHANGE) => {
                let index = self.combo_index(DOCK);
                if let Some(&(_, x, y)) = index.checked_sub(1).and_then(|index| DOCKS.get(index)) {
                    draft.placement.x = x;
                    draft.placement.y = y;
                }
                draft.position = Position::Custom;
                self.draft.set(draft);
            }
            (id, EN_CHANGE) if numeric_field(id) => self.number_changed(id),
            (ANGLE_45, BN_CLICKED) => self.set_number(ANGLE, 45),
            (INK | BACKGROUND, BN_CLICKED) => {
                let color = if id == INK {
                    &mut draft.appearance.ink
                } else {
                    &mut draft.appearance.background
                };
                if let Some(result) = pick_color(window, *color) {
                    *color = result;
                    self.draft.set(draft);
                }
            }
            (APPLY, BN_CLICKED) => {
                self.save(window);
                return;
            }
            (CANCEL, BN_CLICKED) => {
                unsafe { PostMessageW(window, WM_CLOSE, 0, 0) };
                return;
            }
            _ => return,
        }
        let affected = match self.page.get() {
            1 => Some(view::PREVIEW_CARD),
            2 => Some(view::MAP_CARD),
            TIMER_PAGE => Some(view::TIMER_CARDS),
            _ => None,
        };
        if let Some(area) = affected {
            let bounds = view::scaled(area, self.density.get());
            unsafe { InvalidateRect(window, &bounds, 0) };
        }
        unsafe { InvalidateRect(self.control(id), null(), 0) };
    }

    fn save(&self, window: HWND) {
        for (page, ids) in [
            (1, &[SIZE_H, SIZE_M, SIZE_S, TRANSPARENCY][..]),
            (2, &[INSET, ANGLE][..]),
            (TIMER_PAGE, &[WATCH_TRANSPARENCY, COUNT_TRANSPARENCY][..]),
        ] {
            if let Some(&id) = ids.iter().find(|&&id| self.number(id).is_none()) {
                self.change_page(window, page, false);
                unsafe {
                    MessageBoxW(
                        window,
                        w!("A value is outside the allowed range. Correct the highlighted field."),
                        w!("Options"),
                        MB_OK | MB_ICONWARNING,
                    );
                    SetFocus(self.control(id));
                }
                return;
            }
        }
        let enabled = self
            .startup
            .get()
            .filter(|&value| Some(value) != self.original_startup);
        match super::apply_options(self.clock, self.draft.get(), self.original_timers, enabled) {
            Ok(()) => unsafe {
                PostMessageW(window, WM_CLOSE, 0, 0);
            },
            Err(error) => {
                let message = wide(&format!("Could not save settings. {error}"));
                unsafe {
                    MessageBoxW(
                        window,
                        message.as_ptr(),
                        w!("Options"),
                        MB_OK | MB_ICONWARNING,
                    );
                }
            }
        }
    }

    fn map_select(&self, window: HWND, cursor: POINT) -> bool {
        let screens = self.screens.borrow();
        let viewport = view::scaled(view::MAP_VIEW, self.density.get());
        let Some(map) = Map::new(&screens, viewport) else {
            return false;
        };
        let desktop = map.desktop(cursor);
        let Some(display) = screens
            .iter()
            .find(|display| displays::contains(display.area, desktop))
        else {
            return false;
        };
        let mut draft = self.draft.get();
        draft.placement.monitor = display.id;
        draft.position = Position::Custom;
        let size = renderer::stack_size(draft, 96);
        let origin = POINT {
            x: desktop.x - size.cx / 2,
            y: desktop.y - size.cy / 2,
        };
        (draft.placement.x, draft.placement.y) =
            displays::between(display.work, size, draft.placement, origin, 96);
        self.draft.set(draft);
        let index = screens
            .iter()
            .position(|screen| screen.id == display.id)
            .unwrap_or(0);
        drop(screens);
        self.set_combo(SCREEN, index);
        self.sync_dock();
        let bounds = view::scaled(view::MAP_CARD, self.density.get());
        unsafe { InvalidateRect(window, &bounds, 0) };
        true
    }
}

fn numeric_field(id: usize) -> bool {
    matches!(
        id,
        SIZE_H
            | SIZE_M
            | SIZE_S
            | TRANSPARENCY
            | INSET
            | ANGLE
            | WATCH_TRANSPARENCY
            | COUNT_TRANSPARENCY
    )
}

unsafe extern "system" fn combo_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        let original = GetPropW(window, COMBO_PROC);
        if original.is_null() {
            return DefWindowProcW(window, message, wparam, lparam);
        }
        let native: WNDPROC = Some(std::mem::transmute::<
            *mut std::ffi::c_void,
            unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
        >(original));
        if matches!(message, WM_PAINT | WM_PRINTCLIENT | WM_ERASEBKGND) {
            let parent = GetParent(window);
            if let Some(options) =
                (GetWindowLongPtrW(parent, GWLP_USERDATA) as *const Options).as_ref()
            {
                match message {
                    WM_ERASEBKGND => return 1,
                    WM_PRINTCLIENT => options.paint_combo(window, wparam as HDC),
                    _ => {
                        let mut paint = PAINTSTRUCT::default();
                        let dc = BeginPaint(window, &mut paint);
                        options.paint_combo(window, dc);
                        EndPaint(window, &paint);
                    }
                }
                return 0;
            }
        }
        if message == WM_NCDESTROY {
            SetWindowLongPtrW(window, GWLP_WNDPROC, original as isize);
            RemovePropW(window, COMBO_PROC);
        }
        let result = CallWindowProcW(native, window, message, wparam, lparam);
        if matches!(
            message,
            WM_SETFOCUS | WM_KILLFOCUS | CB_SETCURSEL | CB_SHOWDROPDOWN
        ) {
            InvalidateRect(window, null(), 0);
        }
        result
    }
}

fn installed_faces(selected: FaceName) -> Vec<FaceName> {
    let space_mono = FaceName::new("Space Mono");
    let mut faces = vec![space_mono, selected];
    let filters = LOGFONTW {
        lfCharSet: DEFAULT_CHARSET,
        ..Default::default()
    };
    unsafe {
        let dc = GetDC(null_mut());
        if !dc.is_null() {
            EnumFontFamiliesExW(
                dc,
                &filters,
                Some(collect_faces),
                (&raw mut faces) as LPARAM,
                0,
            );
            ReleaseDC(null_mut(), dc);
        }
    }
    let mut named: Vec<_> = faces
        .into_iter()
        .map(|face| (face.name().to_lowercase(), face))
        .collect();
    named.sort_by(|first, second| first.0.cmp(&second.0));
    named.dedup_by(|first, second| first.0 == second.0);
    named.into_iter().map(|(_, face)| face).collect()
}

unsafe extern "system" fn collect_faces(
    font: *const LOGFONTW,
    _: *const TEXTMETRICW,
    kind: u32,
    data: LPARAM,
) -> i32 {
    unsafe {
        if kind & TRUETYPE_FONTTYPE != 0 && (*font).lfFaceName[0] != u16::from(b'@') {
            (&mut *(data as *mut Vec<FaceName>)).push(FaceName((*font).lfFaceName));
        }
    }
    1
}

pub(crate) fn pick_color(window: HWND, current: u32) -> Option<u32> {
    let mut swatches = [0x00ff_ffff; 16];
    let mut dialog = CHOOSECOLORW {
        lStructSize: size_of::<CHOOSECOLORW>() as u32,
        hwndOwner: window,
        rgbResult: view::colorref(current),
        lpCustColors: swatches.as_mut_ptr(),
        Flags: CC_FULLOPEN | CC_RGBINIT,
        ..Default::default()
    };
    unsafe {
        // The native dialog brings in the shell DLLs. Keep them off the idle clock's import path.
        let library = LoadLibraryExW(w!("comdlg32.dll"), null_mut(), LOAD_LIBRARY_SEARCH_SYSTEM32);
        if library.is_null() {
            return None;
        }
        let result =
            GetProcAddress(library, c"ChooseColorW".as_ptr().cast()).is_some_and(|entry| {
                let choose: unsafe extern "system" fn(*mut CHOOSECOLORW) -> i32 =
                    std::mem::transmute(entry);
                choose(&mut dialog) != 0
            });
        FreeLibrary(library);
        result.then(|| view::colorref(dialog.rgbResult))
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

fn screen_name(screen: &Display) -> String {
    let name = screen.id.name();
    format!("Display {}", name.rsplit("DISPLAY").next().unwrap_or(&name))
}

fn point(value: LPARAM) -> POINT {
    POINT {
        x: i32::from(value as i16),
        y: i32::from((value >> 16) as i16),
    }
}
