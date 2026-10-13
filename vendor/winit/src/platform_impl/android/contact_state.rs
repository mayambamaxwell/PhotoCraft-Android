//! Track accepted contacts so rejecting a new palm never cancels an active pen.
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug)]
pub struct Contact {
    pub id: u64,
    pub device: i32,
    pub tool: u32,
    pub x: f64,
    pub y: f64,
}
#[derive(Default)]
pub struct Tracker {
    contacts: BTreeMap<u64, Contact>,
    primary: Option<u64>,
}
pub fn contact_id(device: i32, tool: u32, pointer: i32) -> u64 {
    ((device as u32 as u64) << 32) | ((tool as u64 & 0xffff) << 16) | (pointer as u32 as u64 & 0xffff)
}
impl Tracker {
    pub fn press(&mut self, c: Contact) -> bool {
        if self.contacts.len() >= 32 && !self.contacts.contains_key(&c.id) {
            return false;
        }
        self.contacts.insert(c.id, c);
        if self.primary.is_none() {
            self.primary = Some(c.id);
        }
        true
    }
    pub fn moved(&mut self, c: Contact) -> bool {
        if let Some(current) = self.contacts.get_mut(&c.id) {
            *current = c;
            true
        } else {
            false
        }
    }
    pub fn end(&mut self, id: u64) -> Option<Contact> {
        let contact = self.contacts.remove(&id);
        if self.primary == Some(id) {
            self.primary = None;
        }
        contact
    }
    pub fn reject(&mut self, mut rejected: impl FnMut(u32) -> bool) -> Vec<(Contact, bool)> {
        let ids: Vec<_> = self.contacts.values().filter(|c| rejected(c.tool)).map(|c| c.id).collect();
        ids.into_iter()
            .filter_map(|id| {
                let primary = self.primary == Some(id);
                self.end(id).map(|c| (c, primary))
            })
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn c(device: i32, tool: u32) -> Contact {
        Contact { id: contact_id(device, tool, 0), device, tool, x: 10.0, y: 10.0 }
    }
    #[test]
    fn device_and_tool_ids_do_not_alias() {
        assert_ne!(c(1, 1).id, c(1, 2).id);
        assert_ne!(c(1, 1).id, c(2, 1).id);
    }
    #[test]
    fn rejecting_new_finger_keeps_pen() {
        let mut t = Tracker::default();
        let pen = c(1, 2);
        t.press(pen);
        assert!(t.reject(|tool| tool == 1).is_empty());
        assert!(t.end(c(2, 1).id).is_none());
        assert!(t.moved(pen));
    }
    #[test]
    fn cancel_accepted_finger_before_pen_and_only_once() {
        let mut t = Tracker::default();
        t.press(c(1, 1));
        let removed = t.reject(|tool| tool == 1);
        assert_eq!(removed.len(), 1);
        assert!(removed[0].1);
        assert!(t.reject(|tool| tool == 1).is_empty());
        t.press(c(2, 2));
        t.press(c(1, 1));
        assert!(!t.reject(|tool| tool == 1)[0].1);
    }
}
