use super::Selector;
use ratatui::prelude::*;
use std::borrow::Cow;

pub struct Selection<Key> {
    pub selector: Key,
    pub index: usize,
}

struct SelectorEntry<Key> {
    list: Selector,
    key: Key,
}

/// A collection of selection widgets
/// that can be cycled through
pub struct Selectors<Key> {
    selectors: Vec<SelectorEntry<Key>>,
    focused: Option<Key>,
}

impl<Key> Selectors<Key>
where
    Key: Ord + Clone + Copy,
{
    pub fn new(selectors: &[Key]) -> Self {
        Self {
            focused: selectors.first().copied(),
            selectors: selectors
                .iter()
                .map(|&key| SelectorEntry {
                    list: Selector::default(),
                    key,
                })
                .collect(),
        }
    }

    pub fn focus(&mut self, selector: Key) {
        self.focused = Some(selector);
    }

    pub fn focused(&self) -> Option<Key> {
        self.focused
    }

    pub fn get(&self, selector: Key) -> Option<&Selector> {
        Some(&self.selectors.iter().find(|s| s.key == selector)?.list)
    }

    pub fn get_mut(&mut self, selector: Key) -> Option<&mut Selector> {
        Some(&mut self.find_mut(selector)?.list)
    }

    pub fn next_item(&mut self) {
        if let Some(i) = self.focused_index() {
            self.selectors[i].list.next();
        };
    }

    pub fn previous_item(&mut self) {
        if let Some(i) = self.focused_index() {
            self.selectors[i].list.previous();
        };
    }

    pub fn next_selector(&mut self) {
        if self.selectors.is_empty() {
            return;
        }

        if self.focused.is_none() {
            self.focused = Some(self.selectors.first().unwrap().key);
            return;
        }

        let Some(focused) = self.focused_index() else {
            return;
        };

        let next = (focused + 1) % self.selectors.len();
        self.focused = Some(self.selectors[next].key);
    }

    pub fn previous_selector(&mut self) {
        if self.selectors.is_empty() {
            return;
        }

        if self.focused.is_none() {
            self.focused = Some(self.selectors.first().unwrap().key);
            return;
        }

        let Some(focused) = self.focused_index() else {
            return;
        };

        let previous = focused.wrapping_sub(1) % self.selectors.len();
        self.focused = Some(self.selectors[previous].key);
    }

    pub fn select(&mut self) -> Option<Selection<Key>> {
        let i = self.focused_index()?;
        let selector = &mut self.selectors[i];
        selector.list.confirm_selection();
        Some(Selection {
            selector: selector.key,
            index: selector.list.selected()?,
        })
    }

    pub fn render<'a, B, T>(
        &'a mut self,
        f: &mut Frame<B>,
        area: Rect,
        selector: Key,
        title: &'a str,
        elements: &[T],
    ) where
        for<'b> Cow<'b, str>: From<&'b T>,
        B: Backend,
    {
        let is_focused = self.focused.as_ref() == Some(&selector);

        if let Some(state) = self.find_mut(selector) {
            state.list.render(f, area, title, elements, is_focused);
        };
    }

    fn focused_index(&self) -> Option<usize> {
        let focused = self.focused?;
        self.selectors.iter().position(|s| s.key == focused)
    }

    fn find_mut(&mut self, selector: Key) -> Option<&mut SelectorEntry<Key>> {
        self.selectors.iter_mut().find(|s| s.key == selector)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(strum::EnumIter, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
    enum Sel {
        A,
        B,
        C,
    }

    #[test]
    fn can_cycle_focus_through_selectors_with_default_focus() {
        let mut selectors = Selectors::new(&[Sel::A, Sel::B, Sel::C]);
        assert_eq!(selectors.focused(), Some(Sel::A));

        selectors.next_selector();
        assert_eq!(selectors.focused(), Some(Sel::B));

        selectors.next_selector();
        assert_eq!(selectors.focused(), Some(Sel::C));

        selectors.previous_selector();
        assert_eq!(selectors.focused(), Some(Sel::B));

        selectors.previous_selector();
        assert_eq!(selectors.focused(), Some(Sel::A));
    }

    #[test]
    fn focus_can_wrap_above_and_below_when_cycling() {
        let mut selectors = Selectors::new(&[Sel::A, Sel::B, Sel::C]);

        for _ in 0..100 {
            selectors.next_selector();
        }

        for _ in 0..100 {
            selectors.previous_selector();
        }
    }

    #[test]
    fn focused_selector_can_scroll() {
        let mut selectors = Selectors::new(&[Sel::A, Sel::B, Sel::C]);
        selectors.get_mut(Sel::A).unwrap().resize(2, Some(0));

        let selection = selectors.select().unwrap();
        assert_eq!(selection.selector, Sel::A);
        assert_eq!(selection.index, 0);

        selectors.next_item();

        let selection = selectors.select().unwrap();
        assert_eq!(selection.selector, Sel::A);
        assert_eq!(selection.index, 1);
    }

    #[test]
    fn unfocused_selector_retains_scroll_position() {
        let mut selectors = Selectors::new(&[Sel::A, Sel::B, Sel::C]);
        selectors.get_mut(Sel::A).unwrap().resize(3, Some(0));
        selectors.get_mut(Sel::B).unwrap().resize(3, Some(0));
        selectors.get_mut(Sel::C).unwrap().resize(3, Some(0));

        let selection = selectors.select().unwrap();
        assert_eq!(selection.selector, Sel::A);
        assert_eq!(selection.index, 0);

        selectors.next_item();
        let selection = selectors.select().unwrap();
        assert_eq!(selection.selector, Sel::A);
        assert_eq!(selection.index, 1);

        selectors.next_selector();
        selectors.next_item();
        selectors.next_item();

        let selection = selectors.select().unwrap();
        assert_eq!(selection.selector, Sel::B);
        assert_eq!(selection.index, 2);

        let selection_a = selectors.get(Sel::A).unwrap().selected().unwrap();
        assert_eq!(selection_a, 1);
    }
}
