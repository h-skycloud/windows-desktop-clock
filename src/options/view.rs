use super::*;
use windows_sys::Win32::UI::Controls::{ODS_DISABLED, ODS_FOCUS, ODS_SELECTED};

const PAPER: u32 = 0xffffff;
const PAGE: u32 = 0xf5f4f2;
const SIDEBAR: u32 = 0xfbfaf8;
const SUBTLE: u32 = 0xf4f1ee;
const BORDER: u32 = 0xe7e3de;
const FIELD_BORDER: u32 = 0xd7d2cb;
const TEXT: u32 = 0x272522;
const MUTED: u32 = 0x655f58;
const ACCENT: u32 = 0xf35b28;
const ACCENT_INK: u32 = 0xa73614;
const ACCENT_PAPER: u32 = 0xffeee5;
const ACCENT_BUTTON: u32 = 0xbb421d;

pub const PREVIEW_CARD: RECT = rect(547, 169, 321, 347);
pub const MAP_CARD: RECT = rect(214, 169, 654, 235);
pub const MAP_VIEW: RECT = rect(241, 218, 600, 163);
pub const TIMER_CARDS: RECT = rect(214, 169, 654, 233);

#[derive(Clone, Copy)]
enum Icon {
    Settings = 0xe713,
    Palette = 0xe790,
    Display = 0xe7f4,
    Clock = 0xe917,
    Power = 0xe7e8,
    Eye = 0xe890,
    Rotate = 0xe7ad,
    Check = 0xe73e,
    Down = 0xe70d,
}

pub struct Fonts {
    pub body: HFONT,
    label: HFONT,
    heading: HFONT,
    small: HFONT,
    icon: HFONT,
}

impl Fonts {
    pub fn new(dpi: u32) -> Self {
        Self {
            body: font(w!("Segoe UI"), 15, FW_NORMAL, dpi),
            label: font(w!("Segoe UI"), 15, FW_SEMIBOLD, dpi),
            heading: font(w!("Segoe UI"), 32, FW_SEMIBOLD, dpi),
            small: font(w!("Segoe UI"), 13, FW_NORMAL, dpi),
            icon: font(w!("Segoe Fluent Icons"), 16, FW_NORMAL, dpi),
        }
    }
}

impl Drop for Fonts {
    fn drop(&mut self) {
        for font in [self.body, self.label, self.heading, self.small, self.icon] {
            unsafe {
                DeleteObject(font);
            }
        }
    }
}

fn font(face: PCWSTR, size: i32, weight: u32, dpi: u32) -> HFONT {
    unsafe {
        CreateFontW(
            -scale(size, dpi),
            0,
            0,
            0,
            weight as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET.into(),
            OUT_DEFAULT_PRECIS.into(),
            CLIP_DEFAULT_PRECIS.into(),
            CLEARTYPE_QUALITY.into(),
            FF_SWISS.into(),
            face,
        )
    }
}

impl Options {
    pub(super) fn paint_combo(&self, combo: HWND, dc: HDC) {
        let d = |value| scale(value, self.density.get());
        let mut bounds = RECT::default();
        let mut label = Vec::new();
        let focused = unsafe {
            GetClientRect(combo, &mut bounds);
            let index = SendMessageW(combo, CB_GETCURSEL, 0, 0);
            if index >= 0 {
                let length = SendMessageW(combo, CB_GETLBTEXTLEN, index as usize, 0);
                if length >= 0 {
                    label.resize(length as usize + 1, 0);
                    let count = SendMessageW(
                        combo,
                        CB_GETLBTEXT,
                        index as usize,
                        label.as_mut_ptr() as LPARAM,
                    );
                    label.truncate(count.max(0) as usize);
                }
            }
            GetFocus() == combo || SendMessageW(combo, CB_GETDROPPEDSTATE, 0, 0) != 0
        };
        fill(dc, bounds, PAPER);
        let border = if focused { ACCENT_BUTTON } else { FIELD_BORDER };
        rounded(dc, bounds, d(10), PAPER, border);
        let fonts = self.fonts.borrow();
        let arrow = left_cell(bounds, bounds.right - d(28), d(23));
        glyph(
            dc,
            fonts.icon,
            Icon::Down,
            arrow,
            if focused { ACCENT_INK } else { MUTED },
        );
        let label_bounds = RECT {
            left: d(10),
            right: bounds.right - d(33),
            ..bounds
        };
        text_units(dc, fonts.body, &label, label_bounds, TEXT, DT_LEFT);
    }

    pub(super) fn paint(&self, dc: HDC, damage: RECT) {
        let dpi = self.density.get();
        let fonts = self.fonts.borrow();
        let canvas = Canvas {
            dc,
            dpi,
            fonts: &fonts,
        };
        fill(dc, damage, PAGE);
        canvas.fill(rect(0, 0, WIDTH, 56), PAPER);
        canvas.fill(rect(0, 56, 183, 472), SIDEBAR);
        canvas.fill(rect(0, 528, WIDTH, 57), PAPER);
        canvas.fill(rect(0, 56, WIDTH, 1), BORDER);
        canvas.fill(rect(182, 57, 1, 471), BORDER);
        canvas.fill(rect(0, 528, WIDTH, 1), BORDER);
        canvas.round(rect(21, 13, 30, 30), 10, ACCENT_BUTTON, ACCENT_BUTTON);
        canvas.icon(Icon::Clock, rect(21, 13, 30, 30), PAPER);
        canvas.label("Desktop Clock", rect(63, 14, 220, 28), TEXT);

        let (heading, description) = match self.page.get() {
            0 => ("General", "Windows startup and time display."),
            1 => (
                "Appearance",
                "Preview the clock’s type, colors, and transparency.",
            ),
            2 => ("Position", "Drag the clock or dock it to a screen edge."),
            _ => (
                "Timers & Alarms",
                "Style the timer panels and manage daily alarms.",
            ),
        };
        canvas.text(
            heading,
            rect(214, 77, 654, 38),
            TEXT,
            fonts.heading,
            DT_LEFT,
        );
        canvas.body(description, rect(215, 121, 653, 27), MUTED);
        match self.page.get() {
            0 => {
                canvas.card(rect(214, 169, 654, 162));
                canvas.fill(rect(232, 249, 618, 1), BORDER);
            }
            1 => self.paint_style(&canvas, damage),
            2 => self.paint_placement(&canvas),
            _ => self.paint_timers(&canvas, damage),
        }
        let focused = unsafe { GetFocus() };
        for child in self.controls.borrow().iter() {
            if child.page == Some(self.page.get()) && numeric_field(child.id) {
                let stroke = if child.handle == focused {
                    ACCENT_BUTTON
                } else {
                    FIELD_BORDER
                };
                canvas.round(child.rect, 10, PAPER, stroke);
            }
        }
    }

    fn paint_timers(&self, canvas: &Canvas<'_>, damage: RECT) {
        let settings = self.draft.get();
        for (x, title, style, seconds) in [
            (214, "Stopwatch", settings.timers.stopwatch, 83),
            (
                547,
                "Countdown",
                settings.timers.countdown,
                u64::from(settings.timers.duration),
            ),
        ] {
            canvas.card(rect(x, 169, 321, 233));
            canvas.icon(Icon::Clock, rect(x + 18, 180, 19, 27), ACCENT_INK);
            canvas.label(title, rect(x + 46, 180, 252, 27), TEXT);
            canvas.small("Transparency %", rect(x + 160, 298, 96, 32), MUTED);
            let viewport = rect(x + 20, 340, 281, 49);
            let preview = scaled(viewport, canvas.dpi);
            if intersects(preview, damage) {
                canvas.checker(viewport);
                renderer::preview_panel(
                    canvas.dc,
                    preview,
                    canvas.dpi,
                    settings,
                    renderer::Panel {
                        title,
                        digits: crate::timers::duration_label(seconds),
                        style,
                    },
                );
            }
        }
        canvas.card(rect(214, 413, 654, 105));
        canvas.icon(Icon::Clock, rect(232, 419, 19, 25), ACCENT_INK);
        canvas.label("Daily alarms", rect(258, 419, 590, 25), TEXT);
        canvas.small(
            "Alarms use local time while the clock is running and Windows is awake.",
            rect(232, 485, 610, 24),
            MUTED,
        );
    }

    fn paint_style(&self, canvas: &Canvas<'_>, damage: RECT) {
        canvas.card(rect(214, 169, 311, 347));
        canvas.label("Font", rect(232, 179, 270, 27), TEXT);
        canvas.label("Digit sizes", rect(232, 245, 270, 22), TEXT);
        for (index, label) in ["Hours", "Minutes", "Seconds"].into_iter().enumerate() {
            canvas.small(label, rect(232 + index as i32 * 93, 267, 82, 20), MUTED);
        }
        canvas.label("Background transparency", rect(232, 323, 270, 24), TEXT);
        canvas.small("% · 0–100", rect(320, 356, 180, 32), MUTED);
        canvas.label("Colors", rect(232, 393, 270, 24), TEXT);
        canvas.small(
            "Sizes range from 14 to 96 pixels.",
            rect(232, 473, 270, 27),
            MUTED,
        );

        canvas.card(PREVIEW_CARD);
        canvas.icon(Icon::Eye, rect(565, 179, 19, 27), ACCENT_INK);
        canvas.label("Live preview", rect(593, 179, 255, 27), TEXT);
        let viewport = rect(565, 215, 285, 247);
        if intersects(scaled(viewport, canvas.dpi), damage) {
            canvas.checker(viewport);
            renderer::preview(
                canvas.dc,
                scaled(rect(580, 224, 255, 230), canvas.dpi),
                canvas.dpi,
                self.draft.get(),
            );
        }
        let style = self.draft.get().appearance;
        canvas.color_key(565, "Digits", style.ink);
        canvas.color_key(712, "Background", style.background);
    }

    fn paint_placement(&self, canvas: &Canvas<'_>) {
        canvas.card(MAP_CARD);
        canvas.icon(Icon::Display, rect(232, 179, 18, 29), ACCENT_INK);
        canvas.label("Display layout", rect(258, 179, 583, 29), TEXT);
        canvas.round(rect(231, 209, 620, 177), 12, 0xf0eeea, BORDER);
        let screens = self.screens.borrow();
        let settings = self.draft.get();
        let placement = settings.placement;
        let selected = displays::selected(&screens, placement.monitor);
        if let Some(map) = Map::new(&screens, scaled(MAP_VIEW, canvas.dpi)) {
            for screen in &*screens {
                let bounds = inset(map.project(screen.area), scale(2, canvas.dpi));
                let active = selected.is_some_and(|active| active.id == screen.id);
                rounded(
                    canvas.dc,
                    bounds,
                    scale(10, canvas.dpi),
                    if active { ACCENT_PAPER } else { PAPER },
                    if active { ACCENT_BUTTON } else { FIELD_BORDER },
                );
                let mut label = screen_name(screen);
                if screen.main {
                    label.push_str(" · Primary");
                }
                text(
                    canvas.dc,
                    canvas.fonts.label,
                    &label,
                    bounds,
                    if active { ACCENT_INK } else { MUTED },
                    DT_CENTER,
                );
            }
            if let Some(screen) = selected {
                let size = renderer::stack_size(settings, 96);
                let origin = displays::place(screen.work, size, placement, 96);
                let bounds = map.project(rect(origin.x, origin.y, size.cx, size.cy));
                let display = inset(map.project(screen.area), scale(2, canvas.dpi));
                let width = (bounds.right - bounds.left).max(scale(15, canvas.dpi));
                let height = (bounds.bottom - bounds.top).max(scale(7, canvas.dpi));
                let bounds = fit_marker(bounds, display, width, height);
                rounded(
                    canvas.dc,
                    bounds,
                    scale(6, canvas.dpi),
                    ACCENT_BUTTON,
                    ACCENT_INK,
                );
                let x = (bounds.left + bounds.right) / 2;
                let y = (bounds.top + bounds.bottom) / 2;
                let (sin, cos) = f64::from(placement.angle).to_radians().sin_cos();
                let length = f64::from(
                    (bounds.right - bounds.left)
                        .min(bounds.bottom - bounds.top)
                        .max(7),
                ) / 3.0;
                let dx = (cos * length).round() as i32;
                let dy = (sin * length).round() as i32;
                segment(canvas.dc, (x - dx, y - dy), (x + dx, y + dy), PAPER);
            }
        } else {
            canvas.body(
                "Windows did not report any displays.",
                rect(246, 243, 580, 45),
                MUTED,
            );
        }
        canvas.card(rect(214, 414, 654, 106));
        for (x, width, title) in [
            (232, 162, "Display"),
            (408, 136, "Alignment"),
            (558, 82, "Inset · px"),
            (654, 76, "Angle · °"),
        ] {
            canvas.small(title, rect(x, 421, width, 26), TEXT);
        }
        canvas.small(
            "Set the inset to 0 px to dock flush with the edge.",
            rect(232, 487, 610, 25),
            MUTED,
        );
    }

    pub(super) fn paint_button(&self, button: &DRAWITEMSTRUCT) {
        let id = button.CtlID as usize;
        let state = button.itemState;
        let dc = button.hDC;
        let bounds = button.rcItem;
        let dpi = self.density.get();
        let fonts = self.fonts.borrow();
        let s = |value| scale(value, dpi);
        let draft = self.draft.get();
        let dynamic = match id {
            WATCH_FONT => Some(draft.timers.stopwatch.font.name()),
            COUNT_FONT => Some(draft.timers.countdown.font.name()),
            COUNT_DURATION => Some(String::from_utf16_lossy(&crate::timers::duration_label(
                u64::from(draft.timers.duration),
            ))),
            _ => None,
        };
        let label = dynamic.as_deref().unwrap_or(match id {
            NAV..=103 => ["General", "Appearance", "Position", "Timers"][id - NAV],
            INK => "Digits",
            BACKGROUND => "Background",
            ANGLE_45 => "45°",
            CANCEL => "Cancel",
            APPLY => "Save",
            WATCH_SHOW => {
                if draft.timers.stopwatch.visible {
                    "Panel visible"
                } else {
                    "Panel hidden"
                }
            }
            COUNT_SHOW => {
                if draft.timers.countdown.visible {
                    "Panel visible"
                } else {
                    "Panel hidden"
                }
            }
            WATCH_COLOR | COUNT_COLOR => "Digits",
            WATCH_BACKGROUND | COUNT_BACKGROUND => "Background",
            ALARM_ADD => "Add",
            ALARM_EDIT => "Edit",
            ALARM_TOGGLE => {
                if self.selected_alarm().is_some_and(|alarm| alarm.enabled) {
                    "Disable"
                } else {
                    "Enable"
                }
            }
            ALARM_REMOVE => "Remove",
            _ => "",
        });
        let pressed = state & ODS_SELECTED != 0;
        let focused = state & ODS_FOCUS != 0;
        let nav = (NAV..=103).contains(&id);
        fill(dc, bounds, if nav { SIDEBAR } else { PAPER });
        let border = if focused { ACCENT_BUTTON } else { FIELD_BORDER };
        let mut ink = if state & ODS_DISABLED != 0 {
            MUTED
        } else {
            TEXT
        };
        let mut content = bounds;
        let mut align = DT_CENTER;
        match id {
            NAV..=103 => {
                let active = self.page.get() == id - NAV;
                let background = if active {
                    ACCENT_PAPER
                } else if pressed {
                    SUBTLE
                } else {
                    SIDEBAR
                };
                rounded(
                    dc,
                    bounds,
                    s(9),
                    background,
                    if focused { ACCENT_BUTTON } else { background },
                );
                if active {
                    ink = ACCENT_INK;
                    rounded(
                        dc,
                        rect(bounds.left + s(2), bounds.top + s(10), s(3), s(23)),
                        s(3),
                        ACCENT,
                        ACCENT,
                    );
                }
                let symbol = [Icon::Settings, Icon::Palette, Icon::Display, Icon::Clock][id - NAV];
                glyph(
                    dc,
                    fonts.icon,
                    symbol,
                    inset(left_cell(bounds, s(11), s(24)), s(1)),
                    if active { ACCENT_INK } else { MUTED },
                );
                content.left += s(43);
                align = DT_LEFT;
            }
            STARTUP | SECONDS => {
                let (enabled, symbol, title, description) = if id == STARTUP {
                    (
                        self.startup.get().unwrap_or(false),
                        Icon::Power,
                        "Start with Windows",
                        if self.startup.get().is_some() {
                            "Start the clock when you sign in to Windows."
                        } else {
                            "Windows startup settings are unavailable."
                        },
                    )
                } else {
                    (
                        draft.seconds,
                        Icon::Clock,
                        "Show seconds",
                        "When off, the clock updates once a minute.",
                    )
                };
                let background = if pressed { SUBTLE } else { PAPER };
                rounded(
                    dc,
                    bounds,
                    s(12),
                    background,
                    if focused { ACCENT_BUTTON } else { background },
                );
                let middle = (bounds.top + bounds.bottom) / 2;
                let tile = rect(bounds.left + s(11), middle - s(17), s(34), s(34));
                rounded(dc, tile, s(11), ACCENT_PAPER, ACCENT_PAPER);
                glyph(dc, fonts.icon, symbol, tile, ACCENT_INK);
                let x = bounds.right - s(57);
                let y = middle - s(12);
                let track = if enabled { ACCENT_BUTTON } else { 0xd7d2cb };
                rounded(dc, rect(x, y, s(44), s(24)), s(24), track, track);
                let thumb = if enabled { x + s(23) } else { x + s(3) };
                rounded(dc, rect(thumb, y + s(3), s(18), s(18)), s(18), PAPER, PAPER);
                let ink = if state & ODS_DISABLED != 0 {
                    MUTED
                } else {
                    TEXT
                };
                text(
                    dc,
                    fonts.label,
                    title,
                    rect(bounds.left + s(58), middle - s(22), s(390), s(25)),
                    ink,
                    DT_LEFT,
                );
                text(
                    dc,
                    fonts.small,
                    description,
                    rect(bounds.left + s(58), middle + s(2), s(390), s(20)),
                    MUTED,
                    DT_LEFT,
                );
                text(
                    dc,
                    fonts.small,
                    if enabled { "On" } else { "Off" },
                    rect(x - s(66), middle - s(12), s(55), s(24)),
                    MUTED,
                    DT_RIGHT,
                );
                return;
            }
            INK | BACKGROUND | WATCH_COLOR | COUNT_COLOR | WATCH_BACKGROUND | COUNT_BACKGROUND => {
                rounded(
                    dc,
                    bounds,
                    s(10),
                    if pressed { SUBTLE } else { PAPER },
                    border,
                );
                let color = match id {
                    INK => draft.appearance.ink,
                    BACKGROUND => draft.appearance.background,
                    WATCH_COLOR => draft.timers.stopwatch.color,
                    COUNT_COLOR => draft.timers.countdown.color,
                    WATCH_BACKGROUND => draft
                        .timers
                        .stopwatch
                        .background
                        .unwrap_or(draft.appearance.background),
                    _ => draft
                        .timers
                        .countdown
                        .background
                        .unwrap_or(draft.appearance.background),
                };
                rounded(
                    dc,
                    rect(
                        bounds.left + s(11),
                        (bounds.top + bounds.bottom - s(18)) / 2,
                        s(18),
                        s(18),
                    ),
                    s(18),
                    color,
                    BORDER,
                );
                content.left += s(38);
                align = DT_LEFT;
            }
            WATCH_FONT | COUNT_FONT | COUNT_DURATION => {
                rounded(
                    dc,
                    bounds,
                    s(10),
                    if pressed { SUBTLE } else { PAPER },
                    border,
                );
                glyph(
                    dc,
                    fonts.icon,
                    Icon::Down,
                    left_cell(bounds, bounds.right - s(25), s(20)),
                    MUTED,
                );
                content.left += s(10);
                content.right -= s(26);
                align = DT_LEFT;
            }
            APPLY => {
                let accent = if pressed { ACCENT_INK } else { ACCENT_BUTTON };
                rounded(
                    dc,
                    bounds,
                    s(10),
                    accent,
                    if focused { TEXT } else { accent },
                );
                ink = PAPER;
                glyph(
                    dc,
                    fonts.icon,
                    Icon::Check,
                    left_cell(bounds, s(17), s(19)),
                    PAPER,
                );
                content.left += s(19);
            }
            ANGLE_45 => {
                rounded(
                    dc,
                    bounds,
                    s(10),
                    if pressed { SUBTLE } else { PAPER },
                    border,
                );
                glyph(
                    dc,
                    fonts.icon,
                    Icon::Rotate,
                    left_cell(bounds, s(9), s(18)),
                    ACCENT_INK,
                );
                content.left += s(22);
            }
            _ => {
                rounded(
                    dc,
                    bounds,
                    s(10),
                    if pressed { SUBTLE } else { PAPER },
                    border,
                );
            }
        }
        text(dc, fonts.label, label, content, ink, align);
    }
}

fn left_cell(bounds: RECT, padding: i32, width: i32) -> RECT {
    rect(
        bounds.left + padding,
        bounds.top,
        width,
        bounds.bottom - bounds.top,
    )
}

fn inset(bounds: RECT, space: i32) -> RECT {
    RECT {
        left: bounds.left + space,
        top: bounds.top + space,
        right: bounds.right - space,
        bottom: bounds.bottom - space,
    }
}

fn fit_marker(marker: RECT, display: RECT, width: i32, height: i32) -> RECT {
    let width = width.min((display.right - display.left).max(1));
    let height = height.min((display.bottom - display.top).max(1));
    let x = marker
        .left
        .clamp(display.left, (display.right - width).max(display.left));
    let y = marker
        .top
        .clamp(display.top, (display.bottom - height).max(display.top));
    rect(x, y, width, height)
}

struct Canvas<'a> {
    dc: HDC,
    dpi: u32,
    fonts: &'a Fonts,
}

impl Canvas<'_> {
    fn checker(&self, viewport: RECT) {
        self.fill(viewport, 0xf7f5f1);
        let tile = 19;
        for (row, y) in (viewport.top..viewport.bottom)
            .step_by(tile as usize)
            .enumerate()
        {
            for (column, x) in (viewport.left..viewport.right)
                .step_by(tile as usize)
                .enumerate()
            {
                if (row + column) % 2 == 0 {
                    self.fill(
                        rect(
                            x,
                            y,
                            tile.min(viewport.right - x),
                            tile.min(viewport.bottom - y),
                        ),
                        0xeeece7,
                    );
                }
            }
        }
    }

    fn fill(&self, bounds: RECT, color: u32) {
        fill(self.dc, scaled(bounds, self.dpi), color);
    }

    fn card(&self, bounds: RECT) {
        self.round(bounds, 13, PAPER, BORDER);
    }

    fn round(&self, bounds: RECT, radius: i32, color: u32, border: u32) {
        rounded(
            self.dc,
            scaled(bounds, self.dpi),
            scale(radius, self.dpi),
            color,
            border,
        );
    }

    fn icon(&self, symbol: Icon, bounds: RECT, color: u32) {
        glyph(
            self.dc,
            self.fonts.icon,
            symbol,
            scaled(bounds, self.dpi),
            color,
        );
    }

    fn color_key(&self, x: i32, label: &str, color: u32) {
        self.round(rect(x, 479, 12, 12), 12, color, FIELD_BORDER);
        self.small(
            &format!("{label}  #{color:06X}"),
            rect(x + 18, 475, 130, 22),
            MUTED,
        );
    }

    fn body(&self, label: &str, bounds: RECT, color: u32) {
        self.text(label, bounds, color, self.fonts.body, DT_LEFT);
    }

    fn label(&self, label: &str, bounds: RECT, color: u32) {
        self.text(label, bounds, color, self.fonts.label, DT_LEFT);
    }

    fn small(&self, label: &str, bounds: RECT, color: u32) {
        self.text(label, bounds, color, self.fonts.small, DT_LEFT);
    }

    fn text(&self, label: &str, bounds: RECT, color: u32, font: HFONT, align: u32) {
        text(self.dc, font, label, scaled(bounds, self.dpi), color, align);
    }
}

pub const fn rect(left: i32, top: i32, width: i32, height: i32) -> RECT {
    RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    }
}

pub fn scaled(bounds: RECT, dpi: u32) -> RECT {
    RECT {
        left: scale(bounds.left, dpi),
        top: scale(bounds.top, dpi),
        right: scale(bounds.right, dpi),
        bottom: scale(bounds.bottom, dpi),
    }
}

fn intersects(first: RECT, second: RECT) -> bool {
    first.left < second.right
        && second.left < first.right
        && first.top < second.bottom
        && second.top < first.bottom
}

pub fn colorref(rgb: u32) -> u32 {
    (rgb & 0xff0000) >> 16 | rgb & 0x00ff00 | (rgb & 0x0000ff) << 16
}

fn fill(dc: HDC, bounds: RECT, rgb: u32) {
    unsafe {
        let brush = GetStockObject(DC_BRUSH);
        SetDCBrushColor(dc, colorref(rgb));
        FillRect(dc, &bounds, brush);
    }
}

fn rounded(dc: HDC, bounds: RECT, radius: i32, rgb: u32, border: u32) {
    unsafe {
        let brush = SelectObject(dc, GetStockObject(DC_BRUSH));
        let pen = SelectObject(dc, GetStockObject(DC_PEN));
        SetDCBrushColor(dc, colorref(rgb));
        SetDCPenColor(dc, colorref(border));
        RoundRect(
            dc,
            bounds.left,
            bounds.top,
            bounds.right,
            bounds.bottom,
            radius,
            radius,
        );
        SelectObject(dc, brush);
        SelectObject(dc, pen);
    }
}

fn segment(dc: HDC, from: (i32, i32), to: (i32, i32), color: u32) {
    unsafe {
        let pen = SelectObject(dc, GetStockObject(DC_PEN));
        SetDCPenColor(dc, colorref(color));
        MoveToEx(dc, from.0, from.1, null_mut());
        LineTo(dc, to.0, to.1);
        SelectObject(dc, pen);
    }
}

fn text(dc: HDC, face: HFONT, label: &str, bounds: RECT, rgb: u32, align: u32) {
    let encoded = wide(label);
    text_units(dc, face, &encoded[..encoded.len() - 1], bounds, rgb, align);
}

fn glyph(dc: HDC, face: HFONT, icon: Icon, bounds: RECT, rgb: u32) {
    text_units(dc, face, &[icon as u16], bounds, rgb, DT_CENTER);
}

fn text_units(dc: HDC, face: HFONT, encoded: &[u16], mut bounds: RECT, rgb: u32, align: u32) {
    unsafe {
        let previous = SelectObject(dc, face);
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, colorref(rgb));
        DrawTextW(
            dc,
            encoded.as_ptr(),
            encoded.len() as i32,
            &mut bounds,
            align | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | DT_END_ELLIPSIS,
        );
        SelectObject(dc, previous);
    }
}
