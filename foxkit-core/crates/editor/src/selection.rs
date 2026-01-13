//! Selection management for multi-cursor editing

use crate::cursor::Cursor;

/// A selection in the buffer (anchor to head)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    /// Start of selection (stays fixed during extension)
    pub anchor: Cursor,
    /// End of selection (moves during extension)
    pub head: Cursor,
}

impl Selection {
    /// Create a new selection from anchor to head
    pub fn new(anchor: usize, head: usize) -> Self {
        Self {
            anchor: Cursor::new(anchor),
            head: Cursor::new(head),
        }
    }

    /// Create a point selection (cursor with no selection)
    pub fn point(offset: usize) -> Self {
        Self::new(offset, offset)
    }

    /// Is this a point (no selection)?
    pub fn is_empty(&self) -> bool {
        self.anchor.offset == self.head.offset
    }

    /// Get ordered range (start, end)
    pub fn range(&self) -> (usize, usize) {
        let a = self.anchor.offset;
        let b = self.head.offset;
        if a <= b { (a, b) } else { (b, a) }
    }

    /// Get selection length
    pub fn len(&self) -> usize {
        let (start, end) = self.range();
        end - start
    }

    /// Is selection reversed (head before anchor)?
    pub fn is_reversed(&self) -> bool {
        self.head.offset < self.anchor.offset
    }

    /// Collapse selection to head position
    pub fn collapse(&mut self) {
        self.anchor = self.head;
    }

    /// Extend selection to new head position
    pub fn extend_to(&mut self, offset: usize) {
        self.head.offset = offset;
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self::point(0)
    }
}

/// Set of selections for multi-cursor support
#[derive(Debug, Clone)]
pub struct SelectionSet {
    /// All selections (first is primary)
    selections: Vec<Selection>,
}

impl SelectionSet {
    /// Create a new selection set with cursor at start
    pub fn new() -> Self {
        Self {
            selections: vec![Selection::default()],
        }
    }

    /// Create with a single selection
    pub fn single(selection: Selection) -> Self {
        Self {
            selections: vec![selection],
        }
    }

    /// Get primary selection
    pub fn primary(&self) -> &Selection {
        &self.selections[0]
    }

    /// Get mutable primary selection
    pub fn primary_mut(&mut self) -> &mut Selection {
        &mut self.selections[0]
    }

    /// Add a new selection
    pub fn add(&mut self, selection: Selection) {
        self.selections.push(selection);
        self.normalize();
    }

    /// Clear all but primary selection
    pub fn clear_secondary(&mut self) {
        self.selections.truncate(1);
    }

    /// Number of selections
    pub fn len(&self) -> usize {
        self.selections.len()
    }

    /// Is there only one cursor?
    pub fn is_single(&self) -> bool {
        self.selections.len() == 1
    }

    /// Iterate over selections
    pub fn iter(&self) -> impl Iterator<Item = &Selection> {
        self.selections.iter()
    }

    /// Iterate mutably over selections
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Selection> {
        self.selections.iter_mut()
    }

    /// Merge overlapping selections
    fn normalize(&mut self) {
        // Sort by start position
        self.selections.sort_by_key(|s| s.range().0);
        
        // Merge overlapping
        let mut merged = Vec::with_capacity(self.selections.len());
        
        for sel in &self.selections {
            if let Some(last) = merged.last_mut() {
                let (_, last_end) = last.range();
                let (sel_start, _) = sel.range();
                
                if sel_start <= last_end {
                    // Merge
                    let new_end = sel.range().1.max(last_end);
                    last.head.offset = new_end;
                } else {
                    merged.push(*sel);
                }
            } else {
                merged.push(*sel);
            }
        }
        
        self.selections = merged;
    }
}

impl Default for SelectionSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_range() {
        let sel = Selection::new(5, 10);
        assert_eq!(sel.range(), (5, 10));
        
        let sel_rev = Selection::new(10, 5);
        assert_eq!(sel_rev.range(), (5, 10));
    }

    #[test]
    fn test_multi_cursor() {
        let mut set = SelectionSet::new();
        set.add(Selection::point(10));
        set.add(Selection::point(20));
        
        assert_eq!(set.len(), 3);
    }
}
