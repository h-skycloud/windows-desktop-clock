use crate::timers::{Alarm, MAX_COUNTDOWN, PanelStyle, TimerSettings};
use std::{env, fs, path::PathBuf};

pub const TRANSPARENCY_LEVELS: [u8; 5] = [0, 25, 50, 75, 100];
pub const MIN_DIGIT_SIZE: u16 = 14;
pub const MAX_DIGIT_SIZE: u16 = 96;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    TopCenter,
    TopLeft,
    TopRight,
    BottomCenter,
    BottomLeft,
    BottomRight,
    Last,
    Custom,
}

impl Position {
    pub const ALL: [Self; 7] = [
        Self::TopCenter,
        Self::TopLeft,
        Self::TopRight,
        Self::BottomCenter,
        Self::BottomLeft,
        Self::BottomRight,
        Self::Last,
    ];
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct FaceName(pub [u16; 32]);

impl FaceName {
    pub fn new(name: &str) -> Self {
        let mut face = Self([0; 32]);
        for (slot, unit) in face.0[..31].iter_mut().zip(name.encode_utf16()) {
            *slot = unit;
        }
        face
    }

    pub fn name(self) -> String {
        let end = self.0.iter().position(|&unit| unit == 0).unwrap_or(32);
        String::from_utf16_lossy(&self.0[..end])
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Appearance {
    pub sizes: [u16; 3],
    pub font: FaceName,
    pub ink: u32,
    pub background: u32,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            sizes: [44; 3],
            font: FaceName::new("Space Mono"),
            ink: 0xe5f8e5,
            background: 0x171e25,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub monitor: FaceName,
    pub x: u16,
    pub y: u16,
    pub inset: u16,
    pub angle: i16,
}

impl Default for Placement {
    fn default() -> Self {
        Self {
            monitor: FaceName::default(),
            x: 500,
            y: 0,
            inset: 24,
            angle: 0,
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Settings {
    pub transparency: u8,
    pub seconds: bool,
    pub position: Position,
    pub last_origin: Option<(i32, i32)>,
    pub appearance: Appearance,
    pub placement: Placement,
    pub timers: TimerSettings,
}

impl Settings {
    pub fn load() -> Self {
        let Some(contents) = path().and_then(|path| fs::read_to_string(path).ok()) else {
            return Self::default();
        };
        let mut settings = Self::default();
        for line in contents.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "transparency" => {
                    read_range(value, &mut settings.transparency, 0, 100);
                }
                "seconds" => settings.seconds = value == "true",
                "position" => {
                    if let Ok(index) = value.parse::<usize>() {
                        if let Some(&position) = Position::ALL.get(index) {
                            settings.position = position;
                        } else if index == Position::ALL.len() {
                            settings.position = Position::Custom;
                        }
                    }
                }
                "last_origin" => {
                    if let Some((x, y)) = value.split_once(',')
                        && let (Ok(x), Ok(y)) = (x.parse(), y.parse())
                    {
                        settings.last_origin = Some((x, y));
                    }
                }
                "hours_size" => read_range(
                    value,
                    &mut settings.appearance.sizes[0],
                    MIN_DIGIT_SIZE,
                    MAX_DIGIT_SIZE,
                ),
                "minutes_size" => read_range(
                    value,
                    &mut settings.appearance.sizes[1],
                    MIN_DIGIT_SIZE,
                    MAX_DIGIT_SIZE,
                ),
                "seconds_size" => read_range(
                    value,
                    &mut settings.appearance.sizes[2],
                    MIN_DIGIT_SIZE,
                    MAX_DIGIT_SIZE,
                ),
                "font" if !value.is_empty() => settings.appearance.font = FaceName::new(value),
                "ink" => read_color(value, &mut settings.appearance.ink),
                "background" => read_color(value, &mut settings.appearance.background),
                "monitor" => settings.placement.monitor = FaceName::new(value),
                "x" => read_range(value, &mut settings.placement.x, 0, 1000),
                "y" => read_range(value, &mut settings.placement.y, 0, 1000),
                "inset" => read_range(value, &mut settings.placement.inset, 0, 120),
                "angle" => read_range(value, &mut settings.placement.angle, -180, 180),
                "stopwatch" => read_panel(value, &mut settings.timers.stopwatch),
                "countdown" => read_panel(value, &mut settings.timers.countdown),
                "countdown_duration" => {
                    read_range(value, &mut settings.timers.duration, 1, MAX_COUNTDOWN)
                }
                "alarm" => {
                    let alarms = &mut settings.timers.alarms;
                    if let Some((minute, enabled)) = value.split_once(',')
                        && let (Ok(minute), Ok(enabled)) =
                            (minute.parse::<u16>(), enabled.parse::<bool>())
                        && minute < 1440
                        && !alarms.iter().flatten().any(|alarm| alarm.minute == minute)
                        && let Some(slot) = alarms.iter_mut().find(|entry| entry.is_none())
                    {
                        *slot = Some(Alarm { minute, enabled });
                    }
                }
                _ => {}
            }
        }
        settings
    }

    pub fn save(self) -> std::io::Result<()> {
        let Some(path) = path() else {
            return Err(std::io::ErrorKind::NotFound.into());
        };
        let origin = self
            .last_origin
            .map(|(x, y)| format!("{x},{y}"))
            .unwrap_or_default();
        let position = Position::ALL
            .iter()
            .position(|&position| position == self.position)
            .unwrap_or(Position::ALL.len());
        let appearance = self.appearance;
        let placement = self.placement;
        let mut contents = format!(
            "transparency={}\nseconds={}\nposition={}\nlast_origin={}\nhours_size={}\nminutes_size={}\nseconds_size={}\nfont={}\nink={:06X}\nbackground={:06X}\nmonitor={}\nx={}\ny={}\ninset={}\nangle={}\n",
            self.transparency,
            self.seconds,
            position,
            origin,
            appearance.sizes[0],
            appearance.sizes[1],
            appearance.sizes[2],
            appearance.font.name(),
            appearance.ink,
            appearance.background,
            placement.monitor.name(),
            placement.x,
            placement.y,
            placement.inset,
            placement.angle,
        );
        use std::fmt::Write;
        for (key, panel) in [
            ("stopwatch", self.timers.stopwatch),
            ("countdown", self.timers.countdown),
        ] {
            let _ = writeln!(
                contents,
                "{key}={},{:06X},{}",
                panel.visible,
                panel.color,
                panel.font.name()
            );
        }
        let _ = writeln!(contents, "countdown_duration={}", self.timers.duration);
        for alarm in self.timers.alarms.into_iter().flatten() {
            let _ = writeln!(contents, "alarm={},{}", alarm.minute, alarm.enabled);
        }
        fs::create_dir_all(path.parent().expect("settings directory"))?;
        let temporary = path.with_extension("tmp");
        fs::write(&temporary, contents)?;
        fs::rename(temporary, path)
    }
}

fn read_panel(line: &str, panel: &mut PanelStyle) {
    let mut fields = line.splitn(3, ',');
    if let (Some(visible), Some(color), Some(font)) = (fields.next(), fields.next(), fields.next())
    {
        if let Ok(visible) = visible.parse() {
            panel.visible = visible;
        }
        read_color(color, &mut panel.color);
        if !font.is_empty() {
            panel.font = FaceName::new(font);
        }
    }
}

fn read_range<T: std::str::FromStr + PartialOrd>(text: &str, slot: &mut T, min: T, max: T) {
    if let Ok(value) = text.parse()
        && value >= min
        && value <= max
    {
        *slot = value;
    }
}

fn read_color(text: &str, slot: &mut u32) {
    if text.len() == 6
        && let Ok(value) = u32::from_str_radix(text, 16)
    {
        *slot = value;
    }
}

fn path() -> Option<PathBuf> {
    let app_data = env::var_os("LOCALAPPDATA").filter(|path| !path.is_empty())?;
    Some(PathBuf::from(app_data).join("MasaustuSaati/settings.txt"))
}
