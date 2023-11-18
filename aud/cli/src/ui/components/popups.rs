use crate::ui::widgets;
use ratatui::prelude::*;
use std::collections::BTreeMap;

#[derive(Copy, Clone)]
pub enum PopupKind {
    Code,
    Text,
}

pub struct Popups<Key> {
    popups: BTreeMap<Key, PopupKind>,
    visible: Option<Key>,
}

impl<Key> Popups<Key>
where
    Key: Ord + PartialEq<Key> + Copy,
{
    pub fn new(popups: &[(Key, PopupKind)]) -> Self {
        let popups = popups.iter().fold(BTreeMap::new(), |mut map, popup| {
            map.insert(popup.0, popup.1);
            map
        });

        Self {
            popups,
            visible: None,
        }
    }

    pub fn show(&mut self, popup: Key) {
        self.visible = Some(popup);
    }

    pub fn toggle_visible(&mut self, popup: Key) {
        if self.is_visible(popup) {
            self.hide();
        } else {
            self.show(popup)
        }
    }

    pub fn hide(&mut self) {
        self.visible = None;
    }

    pub fn any_visible(&self) -> bool {
        self.visible.is_some()
    }

    pub fn is_visible(&mut self, popup: Key) -> bool {
        self.visible == Some(popup)
    }

    pub fn render(&mut self, f: &mut Frame, popup: Key, title: &str, text: &str) {
        if self.visible.is_none() {
            return;
        }

        if !self.is_visible(popup) {
            return;
        }

        let Some(popup) = self.popups.iter().find(|p| *p.0 == self.visible.unwrap()) else {
            return;
        };

        match popup.1 {
            PopupKind::Code => widgets::popup::render_code(f, title, text),
            PopupKind::Text => widgets::popup::render_text(f, title, text),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use strum::IntoEnumIterator;

    #[derive(strum::EnumIter, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
    enum Popup {
        A,
        B,
        C,
    }

    #[test]
    fn only_one_popup_is_visible_at_a_time() {
        let mut popups = Popups::new(&[
            (Popup::A, PopupKind::Code),
            (Popup::B, PopupKind::Text),
            (Popup::C, PopupKind::Text),
        ]);

        for popup in Popup::iter() {
            assert!(!popups.is_visible(popup));
        }

        for popup in Popup::iter() {
            popups.show(popup);
            assert!(popups.is_visible(popup));
        }

        popups.hide();

        for popup in Popup::iter() {
            assert!(!popups.is_visible(popup));
        }
    }
}
