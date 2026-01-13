//! Syntax tree wrapper

use tree_sitter::{Tree, Node, Point};

/// A syntax tree
pub struct SyntaxTree {
    inner: Tree,
}

impl SyntaxTree {
    pub fn new(tree: Tree) -> Self {
        Self { inner: tree }
    }

    pub fn inner(&self) -> &Tree {
        &self.inner
    }

    /// Get root node
    pub fn root(&self) -> SyntaxNode {
        SyntaxNode::new(self.inner.root_node())
    }

    /// Edit the tree (for incremental parsing)
    pub fn edit(&mut self, edit: &tree_sitter::InputEdit) {
        self.inner.edit(edit);
    }

    /// Walk the tree
    pub fn walk(&self) -> TreeCursor {
        TreeCursor::new(self.inner.walk())
    }

    /// Find node at point
    pub fn node_at_point(&self, point: Point) -> Option<SyntaxNode> {
        let node = self.inner.root_node().descendant_for_point_range(point, point)?;
        Some(SyntaxNode::new(node))
    }

    /// Find deepest node containing range
    pub fn node_for_range(&self, start: usize, end: usize) -> Option<SyntaxNode> {
        let node = self.inner.root_node().descendant_for_byte_range(start, end)?;
        Some(SyntaxNode::new(node))
    }
}

/// A syntax node
#[derive(Clone, Copy)]
pub struct SyntaxNode<'a> {
    inner: Node<'a>,
}

impl<'a> SyntaxNode<'a> {
    pub fn new(node: Node<'a>) -> Self {
        Self { inner: node }
    }

    /// Get node kind (type name)
    pub fn kind(&self) -> &'static str {
        self.inner.kind()
    }

    /// Get node kind ID
    pub fn kind_id(&self) -> u16 {
        self.inner.kind_id()
    }

    /// Is named node?
    pub fn is_named(&self) -> bool {
        self.inner.is_named()
    }

    /// Is error node?
    pub fn is_error(&self) -> bool {
        self.inner.is_error()
    }

    /// Is missing node?
    pub fn is_missing(&self) -> bool {
        self.inner.is_missing()
    }

    /// Get byte range
    pub fn byte_range(&self) -> std::ops::Range<usize> {
        self.inner.byte_range()
    }

    /// Get start byte
    pub fn start_byte(&self) -> usize {
        self.inner.start_byte()
    }

    /// Get end byte
    pub fn end_byte(&self) -> usize {
        self.inner.end_byte()
    }

    /// Get start position
    pub fn start_position(&self) -> Point {
        self.inner.start_position()
    }

    /// Get end position
    pub fn end_position(&self) -> Point {
        self.inner.end_position()
    }

    /// Get parent node
    pub fn parent(&self) -> Option<SyntaxNode<'a>> {
        self.inner.parent().map(SyntaxNode::new)
    }

    /// Get child count
    pub fn child_count(&self) -> usize {
        self.inner.child_count()
    }

    /// Get child by index
    pub fn child(&self, index: usize) -> Option<SyntaxNode<'a>> {
        self.inner.child(index).map(SyntaxNode::new)
    }

    /// Get child by field name
    pub fn child_by_field(&self, name: &str) -> Option<SyntaxNode<'a>> {
        self.inner.child_by_field_name(name).map(SyntaxNode::new)
    }

    /// Get named children
    pub fn named_children(&self) -> impl Iterator<Item = SyntaxNode<'a>> {
        let mut cursor = self.inner.walk();
        let children: Vec<_> = self.inner.named_children(&mut cursor).collect();
        children.into_iter().map(SyntaxNode::new)
    }

    /// Get all children
    pub fn children(&self) -> impl Iterator<Item = SyntaxNode<'a>> {
        let mut cursor = self.inner.walk();
        let children: Vec<_> = self.inner.children(&mut cursor).collect();
        children.into_iter().map(SyntaxNode::new)
    }

    /// Get node text
    pub fn text<'b>(&self, source: &'b str) -> &'b str {
        &source[self.byte_range()]
    }

    /// Get next sibling
    pub fn next_sibling(&self) -> Option<SyntaxNode<'a>> {
        self.inner.next_sibling().map(SyntaxNode::new)
    }

    /// Get previous sibling
    pub fn prev_sibling(&self) -> Option<SyntaxNode<'a>> {
        self.inner.prev_sibling().map(SyntaxNode::new)
    }
}

impl<'a> std::fmt::Debug for SyntaxNode<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SyntaxNode({} @ {:?})", self.kind(), self.byte_range())
    }
}

/// Tree cursor for walking the tree
pub struct TreeCursor<'a> {
    inner: tree_sitter::TreeCursor<'a>,
}

impl<'a> TreeCursor<'a> {
    pub fn new(cursor: tree_sitter::TreeCursor<'a>) -> Self {
        Self { inner: cursor }
    }

    /// Get current node
    pub fn node(&self) -> SyntaxNode<'a> {
        SyntaxNode::new(self.inner.node())
    }

    /// Go to parent
    pub fn goto_parent(&mut self) -> bool {
        self.inner.goto_parent()
    }

    /// Go to first child
    pub fn goto_first_child(&mut self) -> bool {
        self.inner.goto_first_child()
    }

    /// Go to next sibling
    pub fn goto_next_sibling(&mut self) -> bool {
        self.inner.goto_next_sibling()
    }

    /// Go to first child for byte
    pub fn goto_first_child_for_byte(&mut self, byte: usize) -> Option<usize> {
        self.inner.goto_first_child_for_byte(byte)
    }

    /// Reset to node
    pub fn reset(&mut self, node: SyntaxNode<'a>) {
        self.inner.reset(node.inner);
    }
}
