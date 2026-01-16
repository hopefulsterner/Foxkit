//! Selection and cursor

use smallvec::SmallVec;

/// A selection (anchor to head)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    /// Anchor point (where selection started)
    pub anchor: usize,
    /// Head point (where cursor is)
    pub head: usize,
    /// Affinity (for end-of-line handling)
    pub affinity: Affinity,
}

impl Selection {
    /// Create a cursor (zero-width selection)
    pub fn cursor(offset: usize) -> Self {
        Self {
            anchor: offset,
            head: offset,
            affinity: Affinity::default(),
        }
    }

    /// Create a selection from anchor to head
    pub fn new(anchor: usize, head: usize) -> Self {
        Self {
            anchor,
            head,
            affinity: Affinity::default(),
        }
    }

    /// Get the start offset
    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    /// Get the end offset
    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }

    /// Check if this is a cursor (zero width)
    pub fn is_cursor(&self) -> bool {
        self.anchor == self.head
    }

    /// Check if selection is reversed (head before anchor)
    pub fn is_reversed(&self) -> bool {
        self.head < self.anchor
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.end() - self.start()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get range
    pub fn range(&self) -> std::ops::Range<usize> {
        self.start()..self.end()
    }

    /// Collapse to cursor at head
    pub fn collapse(&self) -> Self {
        Self::cursor(self.head)
    }

    /// Extend selection to offset
    pub fn extend_to(&self, offset: usize) -> Self {
        Self::new(self.anchor, offset)
    }

    /// Move selection by delta
    pub fn translate(&self, delta: isize) -> Self {
        let new_anchor = if delta >= 0 {
            self.anchor + delta as usize
        } else {
            self.anchor.saturating_sub((-delta) as usize)
        };
        let new_head = if delta >= 0 {
            self.head + delta as usize
        } else {
            self.head.saturating_sub((-delta) as usize)
        };
        Self::new(new_anchor, new_head)
    }

    /// Check if offset is within selection
    pub fn contains(&self, offset: usize) -> bool {
        offset >= self.start() && offset < self.end()
    }

    /// Merge with another selection (if overlapping)
    pub fn merge(&self, other: &Selection) -> Option<Selection> {
        if self.start() <= other.end() && other.start() <= self.end() {
            Some(Selection::new(
                self.anchor.min(other.anchor),
                self.head.max(other.head),
            ))
        } else {
            None
        }
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self::cursor(0)
    }
}

/// Cursor affinity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Affinity {
    #[default]
    Upstream,
    Downstream,
}

/// Set of selections (multi-cursor support)
#[derive(Debug, Clone)]
pub struct SelectionSet {
    /// All selections
    selections: SmallVec<[Selection; 4]>,
    /// Index of primary selection
    primary_idx: usize,
}

impl SelectionSet {
    /// Create with single cursor at start
    pub fn new() -> Self {
        Self {
            selections: smallvec::smallvec![Selection::cursor(0)],
            primary_idx: 0,
        }
    }

    /// Create with single selection
    pub fn single(selection: Selection) -> Self {
        Self {
            selections: smallvec::smallvec![selection],
            primary_idx: 0,
        }
    }

    /// Create with multiple selections
    pub fn multiple(selections: impl IntoIterator<Item = Selection>) -> Self {
        let selections: SmallVec<_> = selections.into_iter().collect();
        Self {
            primary_idx: selections.len().saturating_sub(1),
            selections,
        }
    }

    /// Get primary selection
    pub fn primary(&self) -> Selection {
        self.selections.get(self.primary_idx).copied().unwrap_or_default()
    }

    /// Set primary selection index
    pub fn set_primary(&mut self, idx: usize) {
        if idx < self.selections.len() {
            self.primary_idx = idx;
        }
    }

    /// Get all selections
    pub fn all(&self) -> &[Selection] {
        &self.selections
    }

    /// Get number of selections
    pub fn len(&self) -> usize {
        self.selections.len()
    }

    /// Check if empty (shouldn't happen normally)
    pub fn is_empty(&self) -> bool {
        self.selections.is_empty()
    }

    /// Add a selection
    pub fn add(&mut self, selection: Selection) {
        self.selections.push(selection);
        self.primary_idx = self.selections.len() - 1;
        self.normalize();
    }

    /// Remove selection at index
    pub fn remove(&mut self, idx: usize) {
        if self.selections.len() > 1 && idx < self.selections.len() {
            self.selections.remove(idx);
            if self.primary_idx >= self.selections.len() {
                self.primary_idx = self.selections.len() - 1;
            }
        }
    }

    /// Replace all selections
    pub fn set(&mut self, selections: impl IntoIterator<Item = Selection>) {
        self.selections = selections.into_iter().collect();
        if self.selections.is_empty() {
            self.selections.push(Selection::cursor(0));
        }
        self.primary_idx = self.selections.len() - 1;
        self.normalize();
    }

    /// Clear to single cursor
    pub fn clear(&mut self) {
        let primary = self.primary();
        self.selections.clear();
        self.selections.push(primary.collapse());
        self.primary_idx = 0;
    }

    /// Sort and merge overlapping selections
    fn normalize(&mut self) {
        if self.selections.len() <= 1 {
            return;
        }

        // Sort by start position
        self.selections.sort_by_key(|s| s.start());

        // Merge overlapping
        let mut merged = SmallVec::new();
        let mut current = self.selections[0];

        for selection in self.selections.iter().skip(1) {
            if let Some(m) = current.merge(selection) {
                current = m;
            } else {
                merged.push(current);
                current = *selection;
            }
        }
        merged.push(current);

        self.selections = merged;
        self.primary_idx = self.primary_idx.min(self.selections.len() - 1);
    }

    /// Transform selections after edit
    pub fn transform(&mut self, edit_start: usize, old_len: usize, new_len: usize) {
        let delta = new_len as isize - old_len as isize;
        
        for selection in &mut self.selections {
            if selection.anchor >= edit_start + old_len {
                selection.anchor = (selection.anchor as isize + delta) as usize;
            } else if selection.anchor > edit_start {
                selection.anchor = edit_start + new_len;
            }

            if selection.head >= edit_start + old_len {
                selection.head = (selection.head as isize + delta) as usize;
            } else if selection.head > edit_start {
                selection.head = edit_start + new_len;
            }
        }
    }
}

impl Default for SelectionSet {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for SelectionSet {
    type Item = Selection;
    type IntoIter = smallvec::IntoIter<[Selection; 4]>;

    fn into_iter(self) -> Self::IntoIter {
        self.selections.into_iter()
    }
}

impl<'a> IntoIterator for &'a SelectionSet {
    type Item = &'a Selection;
    type IntoIter = std::slice::Iter<'a, Selection>;

    fn into_iter(self) -> Self::IntoIter {
        self.selections.iter()
    }
}
