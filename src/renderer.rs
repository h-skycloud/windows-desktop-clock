use crate::settings::{Appearance, Settings};
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::{HANDLE, HWND, POINT, RECT, SIZE},
    Graphics::Gdi::*,
    UI::WindowsAndMessaging::{ULW_ALPHA, UpdateLayeredWindow},
};

const FONT: &[u8] = include_bytes!("../assets/SpaceMono-Bold.ttf");
const BLEND: BLENDFUNCTION = BLENDFUNCTION {
    BlendOp: AC_SRC_OVER as u8,
    BlendFlags: 0,
    SourceConstantAlpha: 255,
    AlphaFormat: AC_SRC_ALPHA as u8,
};

pub struct ClockFont(HANDLE);

impl ClockFont {
    pub fn load() -> Option<Self> {
        let mut count = 0;
        let handle = unsafe {
            AddFontMemResourceEx(
                FONT.as_ptr().cast(),
                FONT.len() as u32,
                null(),
                &raw mut count,
            )
        };
        (!handle.is_null()).then_some(Self(handle))
    }
}

impl Drop for ClockFont {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { RemoveFontMemResourceEx(self.0) };
        }
    }
}

pub fn size(settings: Settings, dpi: u32) -> SIZE {
    rotated_size(base_size(settings, dpi), settings.placement.angle)
}

fn base_size(settings: Settings, dpi: u32) -> SIZE {
    let sizes = &settings.appearance.sizes[..groups(settings)];
    let total: i32 = sizes.iter().map(|&size| digit_cell(size)).sum();
    let separators: i32 = sizes
        .windows(2)
        .map(|pair| colon_cell(pair[0].min(pair[1])))
        .sum();
    let width = (total + separators + 22).max(if settings.seconds { 270 } else { 200 });
    let height = (i32::from(*sizes.iter().max().unwrap_or(&44)) * 11 / 8 + 22).max(82);
    SIZE {
        cx: super::scale(width, dpi),
        cy: super::scale(height, dpi),
    }
}

fn groups(settings: Settings) -> usize {
    if settings.seconds { 3 } else { 2 }
}

fn digit_cell(size: u16) -> i32 {
    i32::from(size) * 14 / 10
}
fn colon_cell(size: u16) -> i32 {
    i32::from(size) * 63 / 100
}

fn rotated_size(size: SIZE, angle: i16) -> SIZE {
    match angle.rem_euclid(180) {
        0 => size,
        90 => SIZE {
            cx: size.cy,
            cy: size.cx,
        },
        _ => {
            let (sin, cos) = f64::from(angle).to_radians().sin_cos();
            SIZE {
                cx: (f64::from(size.cx) * cos.abs() + f64::from(size.cy) * sin.abs()).ceil() as i32,
                cy: (f64::from(size.cx) * sin.abs() + f64::from(size.cy) * cos.abs()).ceil() as i32,
            }
        }
    }
}

pub fn present(window: HWND, dpi: u32, label: &[u16], settings: Settings) -> bool {
    let Some(surface) = frame(settings, dpi, label) else {
        return false;
    };
    unsafe {
        UpdateLayeredWindow(
            window,
            null_mut(),
            null(),
            &surface.size,
            surface.dc,
            &POINT::default(),
            0,
            &BLEND,
            ULW_ALPHA,
        ) != 0
    }
}

pub fn preview(dc: HDC, bounds: RECT, dpi: u32, settings: Settings) {
    let sample = [50, 51, 58, 52, 56, 58, 53, 54]; // 23:48:56
    let Some(layer) = frame(settings, dpi, &sample[..groups(settings) * 3 - 1]) else {
        return;
    };
    let width = bounds.right - bounds.left;
    let height = bounds.bottom - bounds.top;
    let factor = (f64::from(width) / f64::from(layer.size.cx))
        .min(f64::from(height) / f64::from(layer.size.cy))
        .min(1.0);
    let cx = (f64::from(layer.size.cx) * factor).round() as i32;
    let cy = (f64::from(layer.size.cy) * factor).round() as i32;
    unsafe {
        GdiAlphaBlend(
            dc,
            bounds.left + (width - cx) / 2,
            bounds.top + (height - cy) / 2,
            cx,
            cy,
            layer.dc,
            0,
            0,
            layer.size.cx,
            layer.size.cy,
            BLEND,
        );
    }
}

fn frame(settings: Settings, dpi: u32, label: &[u16]) -> Option<Surface> {
    let dimensions = base_size(settings, dpi);
    let mut surface = Surface::new(dimensions)?;
    let dc = surface.dc;
    unsafe {
        PatBlt(dc, 0, 0, dimensions.cx, dimensions.cy, BLACKNESS);
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, 0x00ff_ffff);
    }
    let sizes = &settings.appearance.sizes[..groups(settings)];
    if sizes.iter().all(|&size| size == sizes[0]) {
        draw(
            dc,
            label,
            0,
            dimensions.cx,
            dimensions.cy,
            dpi,
            sizes[0],
            settings.appearance,
        );
    } else {
        let mut parts = [(0, 0); 5];
        for (index, &size) in sizes.iter().enumerate() {
            parts[index * 2] = (size, super::scale(digit_cell(size), dpi));
            if let Some(next) = sizes.get(index + 1) {
                let small = size.min(*next);
                parts[index * 2 + 1] = (small, super::scale(colon_cell(small), dpi));
            }
        }
        let mut x = (dimensions.cx - parts.iter().map(|&(_, width)| width).sum::<i32>()) / 2;
        let mut offset = 0;
        for (index, &(size, width)) in parts[..sizes.len() * 2 - 1].iter().enumerate() {
            let len = if index % 2 == 0 { 2 } else { 1 };
            draw(
                dc,
                &label[offset..offset + len],
                x,
                width,
                dimensions.cy,
                dpi,
                size,
                settings.appearance,
            );
            x += width;
            offset += len;
        }
    }
    unsafe { GdiFlush() };
    let colors = palette(settings);
    let pixels = surface.pixels_mut();
    for pixel in &mut *pixels {
        *pixel = colors[(*pixel & 0xff) as usize];
    }
    round_corners(
        pixels,
        dimensions.cx as usize,
        dimensions.cy as usize,
        super::scale(12, dpi) as usize,
    );
    if settings.placement.angle == 0 {
        Some(surface)
    } else {
        rotate(&surface, settings.placement.angle)
    }
}

#[allow(clippy::too_many_arguments)]
fn draw(
    dc: HDC,
    label: &[u16],
    left: i32,
    width: i32,
    height: i32,
    dpi: u32,
    font_size: u16,
    appearance: Appearance,
) {
    let font = unsafe {
        CreateFontW(
            -super::scale(i32::from(font_size), dpi),
            0,
            0,
            0,
            FW_BOLD as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET.into(),
            OUT_TT_PRECIS.into(),
            CLIP_DEFAULT_PRECIS.into(),
            ANTIALIASED_QUALITY.into(),
            FF_MODERN.into(),
            appearance.font.0.as_ptr(),
        )
    };
    if font.is_null() {
        return;
    }
    let mut bounds = RECT {
        left,
        right: left + width,
        bottom: height,
        ..Default::default()
    };
    unsafe {
        let old_font = SelectObject(dc, font);
        DrawTextW(
            dc,
            label.as_ptr(),
            label.len() as i32,
            &mut bounds,
            DT_SINGLELINE | DT_CENTER | DT_VCENTER | DT_NOPREFIX | DT_NOCLIP,
        );
        SelectObject(dc, old_font);
        DeleteObject(font);
    }
}

fn palette(settings: Settings) -> [u32; 256] {
    let background_alpha = (100 - u32::from(settings.transparency)) * 255 / 100;
    std::array::from_fn(|coverage| {
        let ink = coverage as u32;
        let backdrop = ((255 - ink) * background_alpha + 127) / 255;
        let channel = |shift: u32| {
            let color = |rgb: u32| (rgb >> shift) & 255;
            ((color(settings.appearance.ink) * ink
                + color(settings.appearance.background) * backdrop
                + 127)
                / 255)
                << shift
        };
        ((ink + backdrop) << 24) | channel(16) | channel(8) | channel(0)
    })
}

fn rotate(source: &Surface, angle: i16) -> Option<Surface> {
    let size = rotated_size(source.size, angle);
    let mut rotated = Surface::new(size)?;
    let pixels = source.pixels();
    let source_width = source.size.cx;
    let source_height = source.size.cy;
    let (sin, cos) = f64::from(angle).to_radians().sin_cos();
    let half_width = f64::from(size.cx - 1) / 2.0;
    let half_height = f64::from(size.cy - 1) / 2.0;
    let source_x = f64::from(source_width - 1) / 2.0;
    let source_y = f64::from(source_height - 1) / 2.0;
    for (row, target) in rotated
        .pixels_mut()
        .chunks_exact_mut(size.cx as usize)
        .enumerate()
    {
        let y = row as f64 - half_height;
        let left_x = -half_width * cos + y * sin + source_x;
        let left_y = half_width * sin + y * cos + source_y;
        for (column, pixel) in target.iter_mut().enumerate() {
            let x = (left_x + column as f64 * cos).round() as i32;
            let y = (left_y - column as f64 * sin).round() as i32;
            *pixel = if (0..source_width).contains(&x) && (0..source_height).contains(&y) {
                pixels[(y * source_width + x) as usize]
            } else {
                0
            };
        }
    }
    Some(rotated)
}

fn round_corners(pixels: &mut [u32], width: usize, height: usize, radius: usize) {
    let radius = radius.min(width / 2).min(height / 2);
    let diameter = radius * 2;
    for y in 0..radius {
        for x in 0..radius {
            let dx = diameter - 2 * x - 1;
            let dy = diameter - 2 * y - 1;
            if dx * dx + dy * dy > diameter * diameter {
                pixels[y * width + x] = 0;
                pixels[y * width + width - 1 - x] = 0;
                pixels[(height - 1 - y) * width + x] = 0;
                pixels[(height - 1 - y) * width + width - 1 - x] = 0;
            }
        }
    }
}

struct Surface {
    size: SIZE,
    dc: HDC,
    bitmap: HBITMAP,
    previous_bitmap: HGDIOBJ,
    bits: *mut u32,
}

impl Surface {
    fn new(size: SIZE) -> Option<Self> {
        let bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size.cx,
                biHeight: -size.cy,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..Default::default()
            },
            ..Default::default()
        };
        unsafe {
            let dc = CreateCompatibleDC(null_mut());
            if dc.is_null() {
                return None;
            }
            let mut bits = null_mut();
            let bitmap =
                CreateDIBSection(dc, &bitmap_info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
            if bitmap.is_null() {
                DeleteDC(dc);
                return None;
            }
            Some(Self {
                size,
                dc,
                bitmap,
                previous_bitmap: SelectObject(dc, bitmap),
                bits: bits.cast(),
            })
        }
    }

    fn pixels(&self) -> &[u32] {
        unsafe { std::slice::from_raw_parts(self.bits, (self.size.cx * self.size.cy) as usize) }
    }

    fn pixels_mut(&mut self) -> &mut [u32] {
        unsafe { std::slice::from_raw_parts_mut(self.bits, (self.size.cx * self.size.cy) as usize) }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous_bitmap);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}
