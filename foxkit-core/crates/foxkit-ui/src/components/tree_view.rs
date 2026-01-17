//! Tree View Component
//!
//! Reusable tree view for file explorer, outline, etc.

/// A node in a tree view
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub label: String,
    pub icon: Option<&'static str>,
    pub expanded: bool,
    pub children: Vec<TreeNode>,
    pub data: Option<String>,
}

impl TreeNode {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            expanded: false,
            children: vec![],
            data: None,
        }
    }

    pub fn with_icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn with_children(mut self, children: Vec<TreeNode>) -> Self {
        self.children = children;
        self
    }

    pub fn expanded(mut self) -> Self {
        self.expanded = true;
        self
    }

    pub fn with_data(mut self, data: impl Into<String>) -> Self {
        self.data = Some(data.into());
        self
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    pub fn toggle(&mut self) {
        if !self.is_leaf() {
            self.expanded = !self.expanded;
        }
    }
}

/// Tree view state
#[derive(Debug, Clone, Default)]
pub struct TreeView {
    pub roots: Vec<TreeNode>,
    pub selected: Option<Vec<usize>>, // Path to selected node
    pub focused: Option<Vec<usize>>,  // Path to focused node
}

impl TreeView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_roots(mut self, roots: Vec<TreeNode>) -> Self {
        self.roots = roots;
        self
    }

    /// Get node at path
    pub fn get_node(&self, path: &[usize]) -> Option<&TreeNode> {
        let mut current: &[TreeNode] = &self.roots;
        let mut node: Option<&TreeNode> = None;

        for &idx in path {
            if idx < current.len() {
                node = Some(&current[idx]);
                current = &current[idx].children;
            } else {
                return None;
            }
        }

        node
    }

    /// Get mutable node at path
    pub fn get_node_mut(&mut self, path: &[usize]) -> Option<&mut TreeNode> {
        let mut current: &mut [TreeNode] = &mut self.roots;
        
        for (i, &idx) in path.iter().enumerate() {
            if idx >= current.len() {
                return None;
            }
            
            if i == path.len() - 1 {
                return Some(&mut current[idx]);
            }
            
            current = &mut current[idx].children;
        }

        None
    }

    /// Toggle node at path
    pub fn toggle(&mut self, path: &[usize]) {
        if let Some(node) = self.get_node_mut(path) {
            node.toggle();
        }
    }

    /// Select node at path
    pub fn select(&mut self, path: Vec<usize>) {
        self.selected = Some(path);
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    /// Flatten visible nodes for rendering
    pub fn flatten(&self) -> Vec<FlatTreeNode> {
        let mut result = Vec::new();
        self.flatten_nodes(&self.roots, 0, &mut vec![], &mut result);
        result
    }

    fn flatten_nodes(
        &self,
        nodes: &[TreeNode],
        depth: usize,
        path: &mut Vec<usize>,
        result: &mut Vec<FlatTreeNode>,
    ) {
        for (i, node) in nodes.iter().enumerate() {
            path.push(i);
            
            let is_selected = self.selected.as_ref() == Some(path);
            let is_focused = self.focused.as_ref() == Some(path);

            result.push(FlatTreeNode {
                label: node.label.clone(),
                icon: node.icon,
                depth,
                is_expanded: node.expanded,
                is_leaf: node.is_leaf(),
                is_selected,
                is_focused,
                path: path.clone(),
                data: node.data.clone(),
            });

            if node.expanded && !node.children.is_empty() {
                self.flatten_nodes(&node.children, depth + 1, path, result);
            }

            path.pop();
        }
    }
}

/// Flattened tree node for rendering
#[derive(Debug, Clone)]
pub struct FlatTreeNode {
    pub label: String,
    pub icon: Option<&'static str>,
    pub depth: usize,
    pub is_expanded: bool,
    pub is_leaf: bool,
    pub is_selected: bool,
    pub is_focused: bool,
    pub path: Vec<usize>,
    pub data: Option<String>,
}

impl FlatTreeNode {
    /// Calculate indent in pixels (typically 16px per level)
    pub fn indent(&self, indent_size: f32) -> f32 {
        self.depth as f32 * indent_size
    }
}
