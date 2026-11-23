use super::*;
use crate::{timer_ui, timers::Alarm};

pub(super) const TIMER_PAGE: usize = 3;
pub(super) const WATCH_SHOW: usize = 140;
pub(super) const WATCH_FONT: usize = 141;
pub(super) const WATCH_COLOR: usize = 142;
pub(super) const COUNT_SHOW: usize = 143;
pub(super) const COUNT_FONT: usize = 144;
pub(super) const COUNT_COLOR: usize = 145;
pub(super) const COUNT_DURATION: usize = 146;
pub(super) const WATCH_BACKGROUND: usize = 147;
pub(super) const COUNT_BACKGROUND: usize = 148;
pub(super) const ALARM_LIST: usize = 150;
pub(super) const ALARM_ADD: usize = 151;
pub(super) const ALARM_EDIT: usize = 152;
pub(super) const ALARM_TOGGLE: usize = 153;
pub(super) const ALARM_REMOVE: usize = 154;
pub(super) const WATCH_TRANSPARENCY: usize = 155;
pub(super) const COUNT_TRANSPARENCY: usize = 156;

impl Options {
    pub(super) fn create_timer_controls(&self, window: HWND) {
        let page = Some(TIMER_PAGE);
        for (id, label, rect) in [
            (
                WATCH_FONT,
                w!("Stopwatch font"),
                view::rect(232, 216, 181, 32),
            ),
            (
                WATCH_COLOR,
                w!("Stopwatch color"),
                view::rect(423, 216, 92, 32),
            ),
            (
                WATCH_SHOW,
                w!("Show stopwatch panel"),
                view::rect(232, 257, 283, 32),
            ),
            (
                COUNT_FONT,
                w!("Countdown font"),
                view::rect(565, 216, 181, 32),
            ),
            (
                COUNT_COLOR,
                w!("Countdown color"),
                view::rect(756, 216, 93, 32),
            ),
            (
                COUNT_SHOW,
                w!("Show countdown panel"),
                view::rect(565, 257, 132, 32),
            ),
            (
                COUNT_DURATION,
                w!("Set countdown duration"),
                view::rect(707, 257, 142, 32),
            ),
            (
                WATCH_BACKGROUND,
                w!("Stopwatch background color"),
                view::rect(232, 298, 134, 32),
            ),
            (
                COUNT_BACKGROUND,
                w!("Countdown background color"),
                view::rect(565, 298, 134, 32),
            ),
            (ALARM_ADD, w!("Add alarm"), view::rect(479, 450, 84, 32)),
            (ALARM_EDIT, w!("Edit alarm"), view::rect(573, 450, 76, 32)),
            (
                ALARM_TOGGLE,
                w!("Enable or disable alarm"),
                view::rect(659, 450, 90, 32),
            ),
            (
                ALARM_REMOVE,
                w!("Remove alarm"),
                view::rect(759, 450, 90, 32),
            ),
        ] {
            self.add(
                window,
                id,
                page,
                w!("BUTTON"),
                label,
                BS_OWNERDRAW as u32,
                rect,
            );
        }
        self.add(
            window,
            ALARM_LIST,
            page,
            w!("COMBOBOX"),
            null(),
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            view::rect(232, 450, 236, 260),
        );
        for (id, x) in [(WATCH_TRANSPARENCY, 471), (COUNT_TRANSPARENCY, 804)] {
            self.add(
                window,
                id,
                page,
                w!("EDIT"),
                null(),
                (ES_NUMBER | ES_AUTOHSCROLL) as u32,
                view::rect(x, 298, 45, 32),
            );
        }
    }

    fn alarm_slot(&self) -> Option<usize> {
        let index = self.combo_index(ALARM_LIST);
        if index == usize::MAX {
            return None;
        }
        let slot = unsafe { SendMessageW(self.control(ALARM_LIST), CB_GETITEMDATA, index, 0) };
        let slot = usize::try_from(slot).ok()?;
        self.draft.get().timers.alarms.get(slot)?.as_ref()?;
        Some(slot)
    }

    pub(super) fn selected_alarm(&self) -> Option<Alarm> {
        self.draft.get().timers.alarms[self.alarm_slot()?]
    }

    pub(super) fn sync_timer_controls(&self, selected: Option<usize>) {
        let settings = self.draft.get().timers;
        let list = self.control(ALARM_LIST);
        let mut selected_row = 0;
        unsafe {
            SendMessageW(list, CB_RESETCONTENT, 0, 0);
        }
        let mut alarms: Vec<_> = settings
            .alarms
            .into_iter()
            .enumerate()
            .filter_map(|(slot, alarm)| Some((slot, alarm?)))
            .collect();
        alarms.sort_by_key(|(_, alarm)| alarm.minute);
        for (slot, alarm) in alarms {
            let label = wide(&format!(
                "{:02}:{:02}  ·  {}",
                alarm.minute / 60,
                alarm.minute % 60,
                if alarm.enabled { "On" } else { "Off" }
            ));
            let row = unsafe { SendMessageW(list, CB_ADDSTRING, 0, label.as_ptr() as LPARAM) };
            if row >= 0 {
                unsafe {
                    SendMessageW(list, CB_SETITEMDATA, row as usize, slot as LPARAM);
                }
                if selected == Some(slot) {
                    selected_row = row as usize;
                }
            }
        }
        unsafe {
            if settings.alarms.iter().all(Option::is_none) {
                SendMessageW(list, CB_ADDSTRING, 0, w!("No alarms set") as LPARAM);
                SendMessageW(list, CB_SETITEMDATA, 0, -1);
            }
            SendMessageW(list, CB_SETCURSEL, selected_row, 0);
            EnableWindow(
                self.control(ALARM_ADD),
                settings.alarms.iter().any(Option::is_none) as i32,
            );
        }
        self.sync_alarm_actions();
    }

    fn sync_alarm_actions(&self) {
        let alarm = self.selected_alarm();
        for id in [ALARM_EDIT, ALARM_TOGGLE, ALARM_REMOVE] {
            unsafe {
                EnableWindow(self.control(id), alarm.is_some() as i32);
                InvalidateRect(self.control(id), null(), 0);
            }
        }
        unsafe {
            SetWindowTextW(
                self.control(ALARM_TOGGLE),
                if alarm.is_some_and(|alarm| alarm.enabled) {
                    w!("Disable alarm")
                } else {
                    w!("Enable alarm")
                },
            );
        }
    }

    pub(super) fn timer_command(&self, window: HWND, id: usize, event: u32) -> bool {
        if id == ALARM_LIST && event == CBN_SELCHANGE {
            self.sync_alarm_actions();
            return true;
        }
        if event != BN_CLICKED
            || !matches!(id, WATCH_SHOW..=COUNT_BACKGROUND | ALARM_ADD..=ALARM_REMOVE)
        {
            return false;
        }
        let mut draft = self.draft.get();
        let timers = &mut draft.timers;
        let mut selected = self.alarm_slot();
        match id {
            WATCH_SHOW => timers.stopwatch.visible = !timers.stopwatch.visible,
            COUNT_SHOW => timers.countdown.visible = !timers.countdown.visible,
            WATCH_FONT | COUNT_FONT => {
                let panel = if id == WATCH_FONT {
                    &mut timers.stopwatch
                } else {
                    &mut timers.countdown
                };
                if let Some(font) = timer_ui::choose_font(window, panel.font) {
                    panel.font = font;
                }
            }
            WATCH_COLOR | COUNT_COLOR => {
                let panel = if id == WATCH_COLOR {
                    &mut timers.stopwatch
                } else {
                    &mut timers.countdown
                };
                if let Some(color) = pick_color(window, panel.color) {
                    panel.color = color;
                }
            }
            WATCH_BACKGROUND | COUNT_BACKGROUND => {
                let panel = if id == WATCH_BACKGROUND {
                    &mut timers.stopwatch
                } else {
                    &mut timers.countdown
                };
                let color = panel.background.unwrap_or(draft.appearance.background);
                if let Some(color) = pick_color(window, color) {
                    panel.background = Some(color);
                }
            }
            COUNT_DURATION => {
                if let Some(duration) = timer_ui::edit_duration(window, timers.duration) {
                    timers.duration = duration;
                }
            }
            ALARM_ADD | ALARM_EDIT => {
                let slot = if id == ALARM_ADD {
                    timers.alarms.iter().position(Option::is_none)
                } else {
                    selected
                };
                if let Some(slot) = slot
                    && let Some(minute) = timer_ui::edit_alarm(window, &timers.alarms, slot)
                {
                    let enabled = timers.alarms[slot].is_none_or(|alarm| alarm.enabled);
                    timers.alarms[slot] = Some(Alarm { minute, enabled });
                    selected = Some(slot);
                }
            }
            ALARM_TOGGLE => {
                if let Some(slot) = selected
                    && let Some(alarm) = &mut timers.alarms[slot]
                {
                    alarm.enabled = !alarm.enabled;
                }
            }
            ALARM_REMOVE => {
                if let Some(slot) = selected {
                    timers.alarms[slot] = None;
                }
            }
            _ => return false,
        }
        self.draft.set(draft);
        if (ALARM_ADD..=ALARM_REMOVE).contains(&id) {
            self.sync_timer_controls(selected);
        }
        unsafe {
            InvalidateRect(self.control(id), null(), 0);
            InvalidateRect(window, null(), 0);
        }
        true
    }
}
