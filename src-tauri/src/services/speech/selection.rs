use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::capture as capture_macos;

pub const POLL_INTERVAL_MS: u64 = 30;
pub const MIN_DRAG_PX: f64 = 3.0;
pub const DOUBLE_CLICK_MS: u64 = 500;
pub const SETTLE_DELAY_MS: u64 = 150;
pub const COPY_WAIT_MS: u64 = 400;

static RUN: AtomicU64 = AtomicU64::new(0);

pub fn begin() -> u64 {
    RUN.fetch_add(1, Ordering::SeqCst) + 1
}

pub fn cancel() {
    RUN.fetch_add(1, Ordering::SeqCst);
}

pub fn is_current(run: u64) -> bool {
    RUN.load(Ordering::SeqCst) == run
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub at_ms: u64,
    pub left_down: bool,
    pub x: f64,
    pub y: f64,
    pub shift: bool,
    pub arrow: bool,
    pub command: bool,
    pub a_key: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Trigger {
    Drag,
    MultiClick,
    ShiftArrow,
    SelectAll,
}

impl Trigger {
    pub fn name(self) -> &'static str {
        match self {
            Trigger::Drag => "drag",
            Trigger::MultiClick => "multi-click",
            Trigger::ShiftArrow => "shift-arrow",
            Trigger::SelectAll => "select-all",
        }
    }
}

#[derive(Default)]
pub struct TriggerDetector {
    pub pill: Rect,
    left_down: bool,
    press: Option<(f64, f64)>,
    last_release_ms: Option<u64>,
    arrow_with_shift: bool,
    a_down: bool,
}

impl TriggerDetector {
    pub fn feed(&mut self, s: &Sample) -> Option<Trigger> {
        let pointer = self.pointer_trigger(s);
        let keys = self.key_trigger(s);
        self.left_down = s.left_down;
        self.a_down = s.a_key;
        pointer.or(keys)
    }

    fn pointer_trigger(&mut self, s: &Sample) -> Option<Trigger> {
        if s.left_down == self.left_down {
            return None;
        }
        if s.left_down {
            self.press = (!self.pill.contains(s.x, s.y)).then_some((s.x, s.y));
            return None;
        }
        let (press_x, press_y) = self.press.take()?;
        let quick = self
            .last_release_ms
            .is_some_and(|last| s.at_ms.saturating_sub(last) <= DOUBLE_CLICK_MS);
        self.last_release_ms = Some(s.at_ms);
        if (s.x - press_x).hypot(s.y - press_y) >= MIN_DRAG_PX {
            Some(Trigger::Drag)
        } else if quick {
            Some(Trigger::MultiClick)
        } else {
            None
        }
    }

    fn key_trigger(&mut self, s: &Sample) -> Option<Trigger> {
        let mut trigger = None;
        if s.shift {
            self.arrow_with_shift |= s.arrow;
        } else {
            if self.arrow_with_shift {
                trigger = Some(Trigger::ShiftArrow);
            }
            self.arrow_with_shift = false;
        }
        if s.command && s.a_key && !self.a_down {
            trigger = Some(Trigger::SelectAll);
        }
        trigger
    }
}

#[derive(Debug, PartialEq)]
pub struct AxSelection<E> {
    pub element: E,
    pub range: Option<(isize, isize)>,
    pub text: String,
}

pub fn equals_baseline<E: PartialEq>(baseline: Option<&AxSelection<E>>, read: &AxSelection<E>) -> bool {
    baseline == Some(read)
}

pub fn should_restore(change_count_now: isize, change_count_after_copy: isize) -> bool {
    change_count_now == change_count_after_copy
}

#[cfg(test)]
mod tests {
    use super::*;

    const PILL: Rect = Rect { x: 100.0, y: 100.0, width: 200.0, height: 60.0 };

    fn sample(at_ms: u64, left_down: bool, x: f64, y: f64) -> Sample {
        Sample { at_ms, left_down, x, y, shift: false, arrow: false, command: false, a_key: false }
    }

    fn detector() -> TriggerDetector {
        TriggerDetector { pill: PILL, ..Default::default() }
    }

    fn drag(detector: &mut TriggerDetector, start_ms: u64, from: (f64, f64), to: (f64, f64)) -> Option<Trigger> {
        assert_eq!(detector.feed(&sample(start_ms, true, from.0, from.1)), None);
        detector.feed(&sample(start_ms + 30, false, to.0, to.1))
    }

    #[test]
    fn selection_drag_of_3_px_outside_the_pill_triggers() {
        assert_eq!(drag(&mut detector(), 0, (500.0, 500.0), (503.0, 500.0)), Some(Trigger::Drag));
    }

    #[test]
    fn selection_drag_of_2_px_does_not_trigger() {
        assert_eq!(drag(&mut detector(), 0, (500.0, 500.0), (502.0, 500.0)), None);
    }

    #[test]
    fn selection_double_click_within_500_ms_triggers() {
        let mut d = detector();
        assert_eq!(drag(&mut d, 0, (500.0, 500.0), (500.0, 500.0)), None);
        assert_eq!(drag(&mut d, 300, (500.0, 500.0), (500.0, 500.0)), Some(Trigger::MultiClick));
    }

    #[test]
    fn selection_two_clicks_600_ms_apart_do_not_trigger() {
        let mut d = detector();
        assert_eq!(drag(&mut d, 0, (500.0, 500.0), (500.0, 500.0)), None);
        assert_eq!(drag(&mut d, 600, (500.0, 500.0), (500.0, 500.0)), None);
    }

    #[test]
    fn selection_press_inside_the_pill_with_a_50_px_drag_does_not_trigger() {
        assert_eq!(drag(&mut detector(), 0, (150.0, 120.0), (150.0, 170.0)), None);
    }

    #[test]
    fn selection_click_in_the_pill_does_not_count_as_the_first_click_of_a_double_click() {
        let mut d = detector();
        assert_eq!(drag(&mut d, 0, (150.0, 120.0), (150.0, 120.0)), None);
        assert_eq!(drag(&mut d, 300, (500.0, 500.0), (500.0, 500.0)), None);
    }

    #[test]
    fn selection_shift_arrow_then_shift_release_triggers() {
        let mut d = detector();
        let held = Sample { shift: true, ..sample(0, false, 0.0, 0.0) };
        assert_eq!(d.feed(&held), None);
        assert_eq!(d.feed(&Sample { arrow: true, ..held }), None);
        assert_eq!(d.feed(&held), None);
        assert_eq!(d.feed(&sample(90, false, 0.0, 0.0)), Some(Trigger::ShiftArrow));
        assert_eq!(d.feed(&sample(120, false, 0.0, 0.0)), None);
    }

    #[test]
    fn selection_shift_release_without_an_arrow_does_not_trigger() {
        let mut d = detector();
        assert_eq!(d.feed(&Sample { shift: true, ..sample(0, false, 0.0, 0.0) }), None);
        assert_eq!(d.feed(&sample(30, false, 0.0, 0.0)), None);
    }

    #[test]
    fn selection_command_a_triggers_once() {
        let mut d = detector();
        let chord = Sample { command: true, a_key: true, ..sample(0, false, 0.0, 0.0) };
        assert_eq!(d.feed(&chord), Some(Trigger::SelectAll));
        assert_eq!(d.feed(&chord), None);
    }

    #[test]
    fn selection_baseline_equal_keeps_waiting() {
        let read = AxSelection { element: 1, range: Some((0, 4)), text: "text".into() };
        let baseline = AxSelection { element: 1, range: Some((0, 4)), text: "text".into() };
        assert!(equals_baseline(Some(&baseline), &read));
    }

    #[test]
    fn selection_baseline_differing_in_text_range_or_element_is_not_equal() {
        let baseline = AxSelection { element: 1, range: Some((0, 4)), text: "text".into() };
        let other_text = AxSelection { element: 1, range: Some((0, 4)), text: "else".into() };
        let other_range = AxSelection { element: 1, range: Some((1, 4)), text: "text".into() };
        let other_element = AxSelection { element: 2, range: Some((0, 4)), text: "text".into() };
        assert!(!equals_baseline(Some(&baseline), &other_text));
        assert!(!equals_baseline(Some(&baseline), &other_range));
        assert!(!equals_baseline(Some(&baseline), &other_element));
    }

    #[test]
    fn selection_without_a_baseline_is_never_equal() {
        let read = AxSelection { element: 1, range: None, text: String::new() };
        assert!(!equals_baseline(None, &read));
    }

    #[test]
    fn selection_pasteboard_is_restored_only_when_the_change_count_is_unchanged() {
        assert!(should_restore(7, 7));
        assert!(!should_restore(8, 7));
    }

    #[test]
    fn selection_run_token_ends_older_runs() {
        let first = begin();
        assert!(is_current(first));
        cancel();
        assert!(!is_current(first));
        let second = begin();
        let third = begin();
        assert!(!is_current(second));
        assert!(is_current(third));
    }
}
