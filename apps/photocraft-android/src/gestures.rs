//! Finger contacts navigate the canvas; they never leak simulated mouse presses into a gesture.
#[cfg(test)]
use egui::Vec2;
use egui::{Event, Pos2, Rect, TouchPhase};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone)]
struct Finger {
    pos: Pos2,
    canvas: bool,
    path: Vec<Pos2>,
    cancelled: bool,
    forwarded: bool,
    reference: bool,
    started: std::time::Instant,
    moved: bool,
}
#[derive(Default)]
pub struct Gestures {
    fingers: BTreeMap<u64, Finger>,
    previous: Option<([u64; 2], [Pos2; 2])>,
    latched: bool,
    pen_active: bool,
    allowed: BTreeSet<u64>,
    tap_group: Option<TapGroup>,
    tap: Option<Tap>,
    pub reference_rect: Option<Rect>,
    pick: Option<Pos2>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tap {
    Undo,
    Redo,
}
struct TapGroup {
    started: std::time::Instant,
    origins: BTreeMap<u64, Pos2>,
    peak: usize,
    valid: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Change {
    pub before: Pos2,
    pub after: Pos2,
    pub scale: f32,
    pub radians: f32,
}
fn pair_change(a: [Pos2; 2], b: [Pos2; 2]) -> Option<Change> {
    let av = a[1] - a[0];
    let bv = b[1] - b[0];
    let length = av.length();
    if length < 8.0 || bv.length() < 8.0 {
        return None;
    }
    let scale = bv.length() / length;
    let angle = (bv.angle() - av.angle() + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    if !scale.is_finite() || !angle.is_finite() {
        return None;
    }
    Some(Change { before: a[0] + av * 0.5, after: b[0] + bv * 0.5, scale: scale.clamp(0.25, 4.0), radians: angle })
}
fn pointer(out: &mut Vec<Event>, pos: Pos2, phase: TouchPhase) {
    let pos = if phase == TouchPhase::Cancel { Pos2::new(-10000., -10000.) } else { pos };
    out.push(Event::PointerMoved(pos));
    match phase {
        TouchPhase::Start | TouchPhase::End => {
            out.push(Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: phase == TouchPhase::Start, modifiers: egui::Modifiers::NONE })
        }
        TouchPhase::Cancel => out.push(Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE }),
        _ => {}
    }
    if matches!(phase, TouchPhase::End | TouchPhase::Cancel) {
        out.push(Event::PointerGone)
    }
}
impl Gestures {
    pub fn filter(&mut self, events: &mut Vec<Event>, canvas: Option<Rect>, pen_only: bool, palm: bool, pen_near: bool) -> Option<Change> {
        let touches: Vec<_> =
            events.iter().filter_map(|e| if let Event::Touch { id, phase, pos, .. } = e { Some((id.0, *phase, *pos)) } else { None }).collect();
        let pen_frame = touches.iter().any(|(id, _, _)| matches!((id >> 16) & 0xffff, 2 | 4));
        if pen_frame || self.pen_active || (palm && pen_near) {
            if let Some(group) = self.tap_group.as_mut() {
                group.valid = false;
            }
        }
        let mut output = Vec::new();
        if palm && pen_near {
            for finger in self.fingers.values_mut() {
                if finger.forwarded {
                    pointer(&mut output, Pos2::new(-10000., -10000.), TouchPhase::Cancel);
                    finger.forwarded = false;
                    finger.cancelled = true;
                }
            }
        }
        if touches.is_empty() {
            self.check_hold(pen_frame || self.pen_active || palm && pen_near);
            events.extend(output);
            return None;
        }

        events.retain(|e| {
            !matches!(e, Event::Touch { .. } | Event::PointerMoved(_) | Event::PointerButton { .. } | Event::PointerGone | Event::Zoom(_) | Event::Rotate(_))
        });
        let mut pen_output = Vec::new();
        let mut ended = Vec::new();
        for (id, phase, pos) in touches {
            if !pos.x.is_finite() || !pos.y.is_finite() {
                continue;
            }
            let tool = (id >> 16) & 0xffff;
            if matches!(tool, 2 | 4) {
                self.pen_active = matches!(phase, TouchPhase::Start | TouchPhase::Move);
            }
            if tool != 1 {
                if phase == TouchPhase::Start && (matches!(tool, 2 | 4) || !pen_only || !canvas.is_some_and(|r| r.contains(pos))) {
                    self.allowed.insert(id);
                }
                if self.allowed.contains(&id) {
                    pointer(&mut pen_output, pos, phase);
                }
                if matches!(phase, TouchPhase::End | TouchPhase::Cancel) {
                    self.allowed.remove(&id);
                }
                continue;
            }
            if phase == TouchPhase::Start && self.fingers.is_empty() {
                self.tap_group = Some(TapGroup {
                    started: std::time::Instant::now(),
                    origins: BTreeMap::new(),
                    peak: 0,
                    valid: !(pen_frame || self.pen_active || palm && pen_near),
                });
            }
            if let Some(group) = self.tap_group.as_mut() {
                if phase == TouchPhase::Start {
                    if group.origins.len() < 10 {
                        group.origins.insert(id, pos);
                    }
                    group.peak = group.peak.max(self.fingers.len().saturating_add(1));
                    if !canvas.is_some_and(|r| r.contains(pos)) || group.peak > 3 || group.origins.len() > 3 {
                        group.valid = false;
                    }
                }
                if phase == TouchPhase::Cancel || group.origins.get(&id).is_some_and(|start| start.distance(pos) > 10.0) {
                    group.valid = false;
                }
            }
            if phase == TouchPhase::Start && self.fingers.len() < 10 {
                let reference = self.reference_rect.is_some_and(|r| r.contains(pos));
                let inside = reference || canvas.is_some_and(|r| r.contains(pos));
                self.fingers.insert(
                    id,
                    Finger {
                        pos,
                        canvas: inside,
                        path: vec![pos],
                        cancelled: palm && pen_near,
                        forwarded: false,
                        reference,
                        started: std::time::Instant::now(),
                        moved: false,
                    },
                );
            }
            if let Some(f) = self.fingers.get_mut(&id) {
                f.moved |= f.path.first().is_some_and(|origin| origin.distance(pos) > 8.);
                f.pos = pos;
                if (palm && pen_near) || phase == TouchPhase::Cancel {
                    f.cancelled = true;
                }
                if f.cancelled && f.forwarded {
                    pointer(&mut output, Pos2::new(-10000., -10000.), TouchPhase::Cancel);
                    f.forwarded = false;
                }
                if f.canvas {
                    if f.path.len() >= 512 {
                        f.path = f.path.iter().step_by(2).copied().collect();
                    }
                    f.path.push(pos);
                } else if !self.latched && !f.cancelled {
                    pointer(&mut output, pos, phase);
                    f.forwarded = matches!(phase, TouchPhase::Start | TouchPhase::Move);
                }
                if matches!(phase, TouchPhase::End | TouchPhase::Cancel) {
                    if phase == TouchPhase::Cancel {
                        f.cancelled = true;
                    }
                    ended.push(id);
                }
            }
        }
        let pair: Vec<_> = self.fingers.iter().filter(|(_, f)| f.canvas && !f.reference).take(2).map(|(&id, f)| (id, f.pos)).collect();
        let mut change = None;
        if pair.len() == 2 && !(palm && (self.pen_active || pen_frame)) {
            for finger in self.fingers.values_mut() {
                if finger.forwarded {
                    pointer(&mut output, Pos2::new(-10000., -10000.), TouchPhase::Cancel);
                    finger.forwarded = false;
                    finger.cancelled = true;
                }
            }
            self.latched = true;
            let ids = [pair[0].0, pair[1].0];
            let positions = [pair[0].1, pair[1].1];
            if let Some((old_ids, old)) = self.previous {
                if ids == old_ids && !self.tap_group.as_ref().is_some_and(|group| group.valid) {
                    change = pair_change(old, positions);
                }
            }
            self.previous = Some((ids, positions));
        } else {
            self.previous = None;
        }
        self.check_hold(pen_frame || self.pen_active || palm && pen_near);
        for id in ended {
            if let Some(f) = self.fingers.remove(&id) {
                if f.canvas && !f.reference && !self.latched && !pen_only && !f.cancelled {
                    if let Some(first) = f.path.first() {
                        pointer(&mut output, *first, TouchPhase::Start);
                        for p in f.path.iter().skip(1) {
                            pointer(&mut output, *p, TouchPhase::Move);
                        }
                        pointer(&mut output, f.pos, TouchPhase::End);
                    }
                }
            }
        }
        if self.fingers.is_empty() {
            if let Some(group) = self.tap_group.take() {
                if group.valid && group.started.elapsed() <= std::time::Duration::from_millis(300) {
                    self.tap = match group.peak {
                        2 => Some(Tap::Undo),
                        3 => Some(Tap::Redo),
                        _ => None,
                    };
                }
            }
            self.latched = false;
            self.previous = None;
        }
        events.extend(output);
        events.extend(pen_output);
        change
    }
    fn check_hold(&mut self, pen: bool) {
        if pen || self.latched || self.fingers.len() != 1 || self.tap_group.as_ref().is_some_and(|g| g.peak != 1) {
            return;
        }
        if let Some(f) = self.fingers.values_mut().next() {
            if f.canvas && !f.cancelled && !f.moved && f.started.elapsed() >= std::time::Duration::from_millis(550) {
                self.pick = Some(f.pos);
                f.cancelled = true;
            }
        }
    }
    pub fn holding(&self) -> bool {
        self.fingers.values().any(|f| f.canvas && !f.cancelled && !f.moved)
    }
    pub fn take_pick(&mut self) -> Option<Pos2> {
        self.pick.take()
    }
    pub fn take_tap(&mut self) -> Option<Tap> {
        self.tap.take()
    }
    pub fn reset(&mut self) {
        self.tap_group = None;
        self.tap = None;
        self.pick = None;
        self.fingers.clear();
        self.previous = None;
        self.latched = false;
        self.allowed.clear();
        self.pen_active = false;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn touch(id: u64, phase: TouchPhase, x: f32, y: f32) -> Event {
        Event::Touch { device_id: egui::TouchDeviceId(1), id: egui::TouchId((1 << 16) | id), phase, pos: Pos2::new(x, y), force: None }
    }
    #[test]
    fn transform_has_pan_zoom_rotation() {
        let c = pair_change([Pos2::ZERO, Pos2::new(20., 0.)], [Pos2::new(10., 10.), Pos2::new(10., 50.)]).unwrap();
        assert!((c.scale - 2.).abs() < 0.001);
        assert!((c.radians - std::f32::consts::FRAC_PI_2).abs() < 0.001);
        assert_eq!(c.after, Pos2::new(10., 30.));
    }
    #[test]
    fn second_finger_and_lifting_never_draw() {
        let mut g = Gestures::default();
        let r = Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(500.)));
        for e in [
            touch(0, TouchPhase::Start, 10., 10.),
            touch(1, TouchPhase::Start, 100., 10.),
            touch(1, TouchPhase::End, 100., 10.),
            touch(0, TouchPhase::Move, 20., 20.),
            touch(0, TouchPhase::End, 20., 20.),
        ] {
            let mut v = vec![e];
            g.filter(&mut v, r, false, false, false);
            assert!(v.is_empty());
        }
    }
    #[test]
    fn pen_only_blocks_finger_stroke_but_ui_taps_work() {
        let mut g = Gestures::default();
        let r = Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(50.)));
        let mut v = vec![touch(0, TouchPhase::Start, 10., 10.), touch(0, TouchPhase::End, 10., 10.)];
        g.filter(&mut v, r, true, false, false);
        assert!(v.is_empty());
        let mut v = vec![touch(0, TouchPhase::Start, 100., 100.), touch(0, TouchPhase::End, 100., 100.)];
        g.filter(&mut v, r, true, false, false);
        assert!(v.iter().any(|e| matches!(e, Event::PointerButton { pressed: true, .. })));
    }
    #[test]
    fn mouse_canvas_is_blocked_but_pen_is_allowed() {
        let mut g = Gestures::default();
        let r = Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.)));
        for tool in [3, 2, 4] {
            let mut events = vec![Event::Touch {
                device_id: egui::TouchDeviceId(1),
                id: egui::TouchId(tool << 16),
                phase: TouchPhase::Start,
                pos: Pos2::new(50., 50.),
                force: Some(0.5),
            }];
            g.filter(&mut events, r, true, false, false);
            assert_eq!(events.iter().any(|e| matches!(e, Event::PointerButton { pressed: true, .. })), tool != 3);
            g.reset();
        }
    }
    #[test]
    fn palm_arrival_cancels_ui_press_outside_screen() {
        let mut g = Gestures::default();
        let mut events = vec![touch(0, TouchPhase::Start, 20., 20.)];
        g.filter(&mut events, None, true, true, false);
        assert!(events.iter().any(|e| matches!(e, Event::PointerButton { pressed: true, .. })));
        let mut events = Vec::new();
        g.filter(&mut events, None, true, true, true);
        assert!(events.iter().any(|e| matches!(e,Event::PointerButton{pressed:false,pos,..} if pos.x<0.0)));
        let mut events = vec![touch(0, TouchPhase::End, 20., 20.)];
        g.filter(&mut events, None, true, true, true);
        assert!(events.is_empty());
    }
    #[test]
    fn long_finger_stroke_is_resampled_not_discarded() {
        let mut g = Gestures::default();
        let canvas = Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(2000.)));
        let mut events = vec![touch(0, TouchPhase::Start, 10., 10.)];
        g.filter(&mut events, canvas, false, false, false);
        for i in 0..700 {
            let mut events = vec![touch(0, TouchPhase::Move, 10. + i as f32, 10.)];
            g.filter(&mut events, canvas, false, false, false);
            assert!(events.is_empty());
        }
        let mut events = vec![touch(0, TouchPhase::End, 710., 10.)];
        g.filter(&mut events, canvas, false, false, false);
        assert!(events.len() < 520);
        assert!(events.iter().any(|e| matches!(e,Event::PointerButton{pos,pressed:false,..} if pos.x==710.)));
    }
}

#[cfg(test)]
mod tap_tests {
    use super::*;
    fn send(g: &mut Gestures, id: u64, phase: TouchPhase, x: f32) {
        let mut events =
            vec![Event::Touch { device_id: egui::TouchDeviceId(1), id: egui::TouchId((1 << 16) | id), phase, pos: Pos2::new(x, 50.), force: None }];
        g.filter(&mut events, Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(500.))), false, false, false);
        assert!(events.is_empty());
    }
    #[test]
    fn movement_cancel_and_long_hold_are_not_taps() {
        for kind in 0..3 {
            let mut g = Gestures::default();
            send(&mut g, 0, TouchPhase::Start, 50.);
            send(&mut g, 1, TouchPhase::Start, 100.);
            if kind == 0 {
                send(&mut g, 0, TouchPhase::Move, 70.);
            }
            if kind == 1 {
                send(&mut g, 0, TouchPhase::Cancel, 50.);
            }
            if kind == 2 {
                g.tap_group.as_mut().unwrap().started -= std::time::Duration::from_millis(400);
            }
            send(&mut g, 0, TouchPhase::End, 50.);
            send(&mut g, 1, TouchPhase::End, 100.);
            assert_eq!(g.take_tap(), None);
        }
    }
    #[test]
    fn three_fingers_wait_for_last_release_and_trigger_once() {
        let mut g = Gestures::default();
        for id in 0..3 {
            send(&mut g, id, TouchPhase::Start, 50. + id as f32 * 30.);
        }
        for id in 0..2 {
            send(&mut g, id, TouchPhase::End, 50. + id as f32 * 30.);
            assert_eq!(g.take_tap(), None);
        }
        send(&mut g, 2, TouchPhase::End, 110.);
        assert_eq!(g.take_tap(), Some(Tap::Redo));
        assert_eq!(g.take_tap(), None);
    }
}

#[cfg(test)]
mod hold_tests {
    use super::*;
    fn event(id: u64, phase: TouchPhase, pos: Pos2) -> Event {
        Event::Touch { device_id: egui::TouchDeviceId(1), id: egui::TouchId((1 << 16) | id), phase, pos, force: None }
    }
    fn send(g: &mut Gestures, events: &mut Vec<Event>) {
        g.filter(events, Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(500., 500.))), true, false, false);
    }
    #[test]
    fn finger_hold_picks_once_and_never_draws() {
        let mut g = Gestures::default();
        let p = Pos2::new(20., 30.);
        send(&mut g, &mut vec![event(0, TouchPhase::Start, p)]);
        g.fingers.values_mut().next().unwrap().started -= std::time::Duration::from_millis(600);
        let mut events = vec![];
        send(&mut g, &mut events);
        assert_eq!(g.take_pick(), Some(p));
        assert!(events.is_empty());
        send(&mut g, &mut events);
        assert_eq!(g.take_pick(), None);
        events.push(event(0, TouchPhase::End, p));
        send(&mut g, &mut events);
        assert!(events.is_empty());
    }
    #[test]
    fn moving_or_two_fingers_cannot_pick() {
        for second in [false, true] {
            let mut g = Gestures::default();
            let p = Pos2::new(20., 30.);
            send(&mut g, &mut vec![event(0, TouchPhase::Start, p)]);
            if second {
                send(&mut g, &mut vec![event(1, TouchPhase::Start, Pos2::new(60., 30.))]);
            } else {
                send(&mut g, &mut vec![event(0, TouchPhase::Move, Pos2::new(80., 30.))]);
            }
            for f in g.fingers.values_mut() {
                f.started -= std::time::Duration::from_secs(1);
            }
            send(&mut g, &mut vec![]);
            assert_eq!(g.take_pick(), None);
        }
    }
    #[test]
    fn reference_hold_does_not_click_or_paint() {
        let mut g = Gestures::default();
        g.reference_rect = Some(Rect::from_min_size(Pos2::new(600., 0.), egui::vec2(200., 200.)));
        let p = Pos2::new(650., 30.);
        let mut v = vec![event(0, TouchPhase::Start, p)];
        send(&mut g, &mut v);
        assert!(v.is_empty());
        g.fingers.values_mut().next().unwrap().started -= std::time::Duration::from_secs(1);
        send(&mut g, &mut v);
        assert_eq!(g.take_pick(), Some(p));
    }
}
