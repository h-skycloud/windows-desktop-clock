use crate::settings::{FaceName, Placement};
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::{LPARAM, POINT, RECT, SIZE},
    Graphics::Gdi::*,
};

#[derive(Clone, Copy)]
pub struct Display {
    pub id: FaceName,
    pub area: RECT,
    pub work: RECT,
    pub main: bool,
}

pub fn connected() -> Vec<Display> {
    let mut displays: Vec<Display> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            null_mut(),
            null(),
            Some(collect),
            (&raw mut displays) as LPARAM,
        )
    };
    displays.sort_by_key(|display| (!display.main, display.area.left, display.area.top));
    displays
}

unsafe extern "system" fn collect(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> i32 {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    unsafe {
        if GetMonitorInfoW(monitor, &raw mut info.monitorInfo) != 0 {
            (&mut *(data as *mut Vec<Display>)).push(Display {
                id: FaceName(info.szDevice),
                area: info.monitorInfo.rcMonitor,
                work: info.monitorInfo.rcWork,
                main: info.monitorInfo.dwFlags & 1 != 0,
            });
        }
    }
    1
}

pub fn selected(displays: &[Display], id: FaceName) -> Option<&Display> {
    displays
        .iter()
        .find(|display| display.id == id)
        .or_else(|| displays.first())
}

pub fn place(area: RECT, size: SIZE, placement: Placement, dpi: u32) -> POINT {
    let inset = super::scale(i32::from(placement.inset), dpi);
    let x = range(area.left, area.right, size.cx, inset);
    let y = range(area.top, area.bottom, size.cy, inset);
    POINT {
        x: x.0 + (x.1 - x.0) * i32::from(placement.x) / 1000,
        y: y.0 + (y.1 - y.0) * i32::from(placement.y) / 1000,
    }
}

pub fn between(
    area: RECT,
    size: SIZE,
    placement: Placement,
    origin: POINT,
    dpi: u32,
) -> (u16, u16) {
    let inset = super::scale(i32::from(placement.inset), dpi);
    let fraction = |value: i32, (low, high): (i32, i32)| {
        if low == high {
            500
        } else {
            ((value.clamp(low, high) - low) * 1000 / (high - low)) as u16
        }
    };
    (
        fraction(origin.x, range(area.left, area.right, size.cx, inset)),
        fraction(origin.y, range(area.top, area.bottom, size.cy, inset)),
    )
}

fn range(low: i32, high: i32, size: i32, inset: i32) -> (i32, i32) {
    let travel = (high - low - size).max(0);
    let inset = inset.min(travel / 2);
    (low + inset, low + travel - inset)
}

pub struct Map {
    factor: f64,
    left: f64,
    top: f64,
}

impl Map {
    pub fn new(displays: &[Display], viewport: RECT) -> Option<Self> {
        let first = displays.first()?.area;
        let extents = displays.iter().fold(first, |outer, display| RECT {
            left: outer.left.min(display.area.left),
            top: outer.top.min(display.area.top),
            right: outer.right.max(display.area.right),
            bottom: outer.bottom.max(display.area.bottom),
        });
        let width = f64::from((extents.right - extents.left).max(1));
        let height = f64::from((extents.bottom - extents.top).max(1));
        let target_width = f64::from((viewport.right - viewport.left).max(1));
        let target_height = f64::from((viewport.bottom - viewport.top).max(1));
        let factor = (target_width / width).min(target_height / height);
        Some(Self {
            factor,
            left: f64::from(viewport.left) + (target_width - width * factor) / 2.0
                - f64::from(extents.left) * factor,
            top: f64::from(viewport.top) + (target_height - height * factor) / 2.0
                - f64::from(extents.top) * factor,
        })
    }

    pub fn project(&self, rect: RECT) -> RECT {
        let x = |value| (self.left + f64::from(value) * self.factor).round() as i32;
        let y = |value| (self.top + f64::from(value) * self.factor).round() as i32;
        RECT {
            left: x(rect.left),
            top: y(rect.top),
            right: x(rect.right),
            bottom: y(rect.bottom),
        }
    }

    pub fn desktop(&self, point: POINT) -> POINT {
        POINT {
            x: ((f64::from(point.x) - self.left) / self.factor).round() as i32,
            y: ((f64::from(point.y) - self.top) / self.factor).round() as i32,
        }
    }
}

pub fn contains(rect: RECT, point: POINT) -> bool {
    (rect.left..rect.right).contains(&point.x) && (rect.top..rect.bottom).contains(&point.y)
}
