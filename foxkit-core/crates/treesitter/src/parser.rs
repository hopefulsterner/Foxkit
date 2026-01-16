//! Parser wrapper

use crate::Language;

/// Tree-sitter parser wrapper
pub struct Parser {
    inner: tree_sitter::Parser,
    language: Language,
}

impl Parser {
    /// Create a new parser for language
    pub fn new(language: Language) -> Self {
        let mut inner = tree_sitter::Parser::new();
        inner.set_language(&language.ts_language()).expect("Failed to set language");
        
        Self { inner, language }
    }

    /// Parse source code
    pub fn parse(&self, source: &str, old_tree: Option<&Tree>) -> Option<Tree> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&self.language.ts_language()).ok()?;
        
        let ts_tree = parser.parse(source, old_tree.map(|t| &t.inner))?;
        Some(Tree { inner: ts_tree })
    }

    /// Get language
    pub fn language(&self) -> Language {
        self.language
    }
}

/// Syntax tree
pub struct Tree {
    inner: tree_sitter::Tree,
}

impl Tree {
    /// Get root node
    pub fn root_node(&self) -> Node<'_> {
        Node { inner: self.inner.root_node() }
    }

    /// Walk the tree
    pub fn walk(&self) -> TreeCursor<'_> {
        TreeCursor { inner: self.inner.walk() }
    }

    /// Edit the tree (for incremental parsing)
    pub fn edit(&mut self, edit: &InputEdit) {
        self.inner.edit(&tree_sitter::InputEdit {
            start_byte: edit.start_byte,
            old_end_byte: edit.old_end_byte,
            new_end_byte: edit.new_end_byte,
            start_position: tree_sitter::Point {
                row: edit.start_position.0,
                column: edit.start_position.1,
            },
            old_end_position: tree_sitter::Point {
                row: edit.old_end_position.0,
                column: edit.old_end_position.1,
            },
            new_end_position: tree_sitter::Point {
                row: edit.new_end_position.0,
                column: edit.new_end_position.1,
            },
        });
    }
}

/// Syntax node
#[derive(Clone, Copy)]
pub struct Node<'a> {
    pub(crate) inner: tree_sitter::Node<'a>,
}

impl<'a> Node<'a> {
    /// Get node kind (type)
    pub fn kind(&self) -> &'static str {
        self.inner.kind()
    }

    /// Is this a named node?
    pub fn is_named(&self) -> bool {
        self.inner.is_named()
    }

    /// Is this an error node?
    pub fn is_error(&self) -> bool {
        self.inner.is_error()
    }

    /// Is this a missing node?
    pub fn is_missing(&self) -> bool {
        self.inner.is_missing()
    }

    /// Get start byte offset
    pub fn start_byte(&self) -> usize {
        self.inner.start_byte()
    }

    /// Get end byte offset
    pub fn end_byte(&self) -> usize {
        self.inner.end_byte()
    }

    /// Get start position (row, column)
    pub fn start_position(&self) -> Position {
        let p = self.inner.start_position();
        Position { row: p.row, column: p.column }
    }

    /// Get end position (row, column)
    pub fn end_position(&self) -> Position {
        let p = self.inner.end_position();
        Position { row: p.row, column: p.column }
    }

    /// Get parent node
    pub fn parent(&self) -> Option<Node<'a>> {
        self.inner.parent().map(|n| Node { inner: n })
    }

    /// Get child count
    pub fn child_count(&self) -> usize {
        self.inner.child_count()
    }

    /// Get child by index
    pub fn child(&self, index: usize) -> Option<Node<'a>> {
        self.inner.child(index).map(|n| Node { inner: n })
    }

    /// Get named child by index
    pub fn named_child(&self, index: usize) -> Option<Node<'a>> {
        self.inner.named_child(index).map(|n| Node { inner: n })
    }

    /// Get child by field name
    pub fn child_by_field_name(&self, field: &str) -> Option<Node<'a>> {
        self.inner.child_by_field_name(field).map(|n| Node { inner: n })
    }

    /// Iterate children
    pub fn children<'b>(&'b self, cursor: &'b mut TreeCursor<'a>) -> impl Iterator<Item = Node<'a>> + 'b {
        cursor.inner.reset(self.inner);
        cursor.inner.goto_first_child();
        
        std::iter::from_fn(move || {
            let node = Node { inner: cursor.inner.node() };
            if cursor.inner.goto_next_sibling() {
                Some(node)
            } else {
                None
            }
        })
    }

    /// Get node text
    pub fn text<'b>(&self, source: &'b str) -> &'b str {
        &source[self.start_byte()..self.end_byte()]
    }
}

impl std::fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("kind", &self.kind())
            .field("start", &self.start_byte())
            .field("end", &self.end_byte())
            .finish()
    }
}

/// Tree cursor for traversal
pub struct TreeCursor<'a> {
    inner: tree_sitter::TreeCursor<'a>,
}

impl<'a> TreeCursor<'a> {
    /// Get current node
    pub fn node(&self) -> Node<'a> {
        Node { inner: self.inner.node() }
    }

    /// Go to first child
    pub fn goto_first_child(&mut self) -> bool {
        self.inner.goto_first_child()
    }

    /// Go to next sibling
    pub fn goto_next_sibling(&mut self) -> bool {
        self.inner.goto_next_sibling()
    }

    /// Go to parent
    pub fn goto_parent(&mut self) -> bool {
        self.inner.goto_parent()
    }

    /// Reset to node
    pub fn reset(&mut self, node: Node<'a>) {
        self.inner.reset(node.inner);
    }
}

/// Position in source
#[derive(Debug, Clone, Copy, Default)]
pub struct Position {
    pub row: usize,
    pub column: usize,
}

/// Input edit for incremental parsing
pub struct InputEdit {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_position: (usize, usize),
    pub old_end_position: (usize, usize),
    pub new_end_position: (usize, usize),
}
