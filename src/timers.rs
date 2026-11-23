use crate::settings::FaceName;
use windows_sys::Win32::Foundation::SYSTEMTIME;

pub const ALARM_LIMIT: usize = 12;
pub const MAX_COUNTDOWN: u32 = 359_999;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PanelStyle {
    pub visible: bool,
    pub font: FaceName,
    pub color: u32,
    pub background: Option<u32>,
    pub transparency: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Alarm {
    pub minute: u16,
    pub enabled: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TimerSettings {
    pub stopwatch: PanelStyle,
    pub countdown: PanelStyle,
    pub duration: u32,
    pub alarms: [Option<Alarm>; ALARM_LIMIT],
}

impl Default for TimerSettings {
    fn default() -> Self {
        let panel = PanelStyle {
            visible: true,
            font: FaceName::new("Space Mono"),
            color: 0xe5f8e5,
            background: None,
            transparency: 0,
        };
        Self {
            stopwatch: panel,
            countdown: PanelStyle {
                color: 0xffce95,
                ..panel
            },
            duration: 300,
            alarms: [None; ALARM_LIMIT],
        }
    }
}

impl TimerSettings {
    pub fn merge(self, original: Self, latest: Self) -> std::io::Result<Self> {
        let alarms_changed = self.alarms != original.alarms;
        if alarms_changed && latest.alarms != original.alarms && latest.alarms != self.alarms {
            return Err(std::io::Error::other(
                "Alarms changed from the clock menu. Reopen Options before editing them.",
            ));
        }
        Ok(Self {
            stopwatch: merge_panel(self.stopwatch, original.stopwatch, latest.stopwatch),
            countdown: merge_panel(self.countdown, original.countdown, latest.countdown),
            duration: keep_edit(self.duration, original.duration, latest.duration),
            alarms: if alarms_changed {
                self.alarms
            } else {
                latest.alarms
            },
        })
    }
}

fn merge_panel(draft: PanelStyle, original: PanelStyle, latest: PanelStyle) -> PanelStyle {
    PanelStyle {
        visible: keep_edit(draft.visible, original.visible, latest.visible),
        font: keep_edit(draft.font, original.font, latest.font),
        color: keep_edit(draft.color, original.color, latest.color),
        background: keep_edit(draft.background, original.background, latest.background),
        transparency: keep_edit(
            draft.transparency,
            original.transparency,
            latest.transparency,
        ),
    }
}

fn keep_edit<T: Eq>(draft: T, original: T, latest: T) -> T {
    if draft == original { latest } else { draft }
}

#[derive(Clone, Copy, Default)]
pub struct Stopwatch {
    elapsed: u64,
    started: Option<u64>,
}

impl Stopwatch {
    pub fn elapsed(self, now: u64) -> u64 {
        self.elapsed
            + self
                .started
                .map_or(0, |started| now.saturating_sub(started))
    }

    pub fn running(self) -> bool {
        self.started.is_some()
    }

    pub fn toggle(&mut self, now: u64) {
        if self.running() {
            self.elapsed = self.elapsed(now);
            self.started = None;
        } else {
            self.started = Some(now);
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Countdown {
    remaining: Option<u64>,
    deadline: Option<u64>,
}

impl Countdown {
    pub fn remaining(self, now: u64, duration: u32) -> u64 {
        self.deadline.map_or_else(
            || self.remaining.unwrap_or(u64::from(duration) * 1000),
            |end| end.saturating_sub(now),
        )
    }

    pub fn running(self) -> bool {
        self.deadline.is_some()
    }
    pub fn finished(self) -> bool {
        self.remaining == Some(0) && !self.running()
    }

    pub fn toggle(&mut self, now: u64, duration: u32) {
        if self.running() {
            self.remaining = Some(self.remaining(now, duration));
            self.deadline = None;
        } else {
            let remaining = self.remaining(now, duration);
            self.deadline = Some(
                now + if remaining == 0 {
                    u64::from(duration) * 1000
                } else {
                    remaining
                },
            );
        }
    }

    pub fn finish_if_due(&mut self, now: u64) -> bool {
        if self.deadline.is_some_and(|deadline| deadline <= now) {
            self.remaining = Some(0);
            self.deadline = None;
            return true;
        }
        false
    }
}

#[derive(Default)]
pub struct AlarmHistory([u32; ALARM_LIMIT]);

impl AlarmHistory {
    pub fn due(
        &mut self,
        local: &SYSTEMTIME,
        alarms: &[Option<Alarm>; ALARM_LIMIT],
    ) -> Option<u16> {
        let today = day(local);
        let minute = local.wHour * 60 + local.wMinute;
        let index = alarms.iter().enumerate().position(|(index, alarm)| {
            self.0[index] != today
                && alarm.is_some_and(|alarm| alarm.enabled && alarm.minute == minute)
        })?;
        self.0[index] = today;
        Some(minute)
    }

    pub fn changed(&mut self, slot: usize, local: &SYSTEMTIME, minute: u16) {
        self.0[slot] = if local.wHour * 60 + local.wMinute == minute {
            day(local)
        } else {
            0
        };
    }
}

fn day(local: &SYSTEMTIME) -> u32 {
    u32::from(local.wYear) * 10_000 + u32::from(local.wMonth) * 100 + u32::from(local.wDay)
}

pub fn duration_label(seconds: u64) -> [u16; 8] {
    let seconds = seconds.min(u64::from(MAX_COUNTDOWN));
    let digit = |number| u16::from(b'0') + (number % 10) as u16;
    let hours = seconds / 3600;
    [
        digit(hours / 10),
        digit(hours),
        58,
        digit(seconds / 600 % 6),
        digit(seconds / 60),
        58,
        digit(seconds / 10 % 6),
        digit(seconds),
    ]
}

pub fn parse_duration(text: &str) -> Option<u32> {
    let (hours, rest) = text.trim().split_once(':')?;
    let (minutes, seconds) = rest.split_once(':')?;
    let (hours, minutes, seconds) = (field(hours, 100)?, field(minutes, 60)?, field(seconds, 60)?);
    let total = hours * 3600 + minutes * 60 + seconds;
    (total != 0).then_some(total)
}

pub fn parse_alarm(text: &str) -> Option<u16> {
    let (hours, minutes) = text.trim().split_once(':')?;
    Some((field(hours, 24)? * 60 + field(minutes, 60)?) as u16)
}

fn field(text: &str, limit: u32) -> Option<u32> {
    if text.is_empty() || text.len() > 2 || !text.bytes().all(|digit| digit.is_ascii_digit()) {
        return None;
    }
    let value = text.parse().ok()?;
    (value < limit).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_preserve_unedited_timer_fields_and_detect_alarm_conflicts() {
        let original = TimerSettings::default();
        let mut draft = original;
        let mut latest = original;
        draft.stopwatch.visible = false;
        latest.stopwatch.color = 0x123456;
        latest.alarms[0] = Some(Alarm {
            minute: 7 * 60,
            enabled: true,
        });
        let merged = draft.merge(original, latest).unwrap();
        assert!(!merged.stopwatch.visible);
        assert_eq!(merged.stopwatch.color, 0x123456);
        assert_eq!(merged.alarms[0].unwrap().minute, 7 * 60);
        draft.alarms[0] = Some(Alarm {
            minute: 8 * 60,
            enabled: false,
        });
        assert!(draft.merge(original, latest).is_err());
    }

    #[test]
    fn stopwatch_excludes_paused_time() {
        let mut watch = Stopwatch::default();
        watch.toggle(100);
        watch.toggle(1600);
        assert_eq!(watch.elapsed(7000), 1500);
        watch.toggle(9000);
        assert_eq!(watch.elapsed(9500), 2000);
    }

    #[test]
    fn countdown_resumes_and_alerts_once() {
        let mut timer = Countdown::default();
        timer.toggle(100, 4);
        timer.toggle(1100, 4);
        assert_eq!(timer.remaining(9000, 4), 3000);
        timer.toggle(10_000, 4);
        assert!(!timer.finish_if_due(12_999));
        assert!(timer.finish_if_due(13_000));
        assert!(!timer.finish_if_due(14_000));
        timer.toggle(15_000, 4);
        assert_eq!(timer.remaining(15_000, 4), 4000);
    }

    #[test]
    fn daily_alarm_ignores_duplicate_ticks_and_disabled_entries() {
        let mut local = SYSTEMTIME {
            wYear: 2026,
            wMonth: 11,
            wDay: 23,
            wHour: 8,
            ..Default::default()
        };
        let mut alarms = [None; ALARM_LIMIT];
        alarms[0] = Some(Alarm {
            minute: 480,
            enabled: false,
        });
        alarms[1] = Some(Alarm {
            minute: 480,
            enabled: true,
        });
        let mut history = AlarmHistory::default();
        assert_eq!(history.due(&local, &alarms), Some(480));
        assert_eq!(history.due(&local, &alarms), None);
        local.wDay += 1;
        assert_eq!(history.due(&local, &alarms), Some(480));
    }

    #[test]
    fn input_and_display_ranges() {
        assert_eq!(parse_duration("99:59:59"), Some(MAX_COUNTDOWN));
        assert_eq!(parse_duration("0:00:00"), None);
        assert_eq!(parse_duration("0:60:00"), None);
        assert_eq!(parse_alarm("23:59"), Some(1439));
        assert_eq!(parse_alarm("24:00"), None);
        assert_eq!(String::from_utf16_lossy(&duration_label(3669)), "01:01:09");
        assert_eq!(
            String::from_utf16_lossy(&duration_label(360_000)),
            "99:59:59"
        );
    }
}
