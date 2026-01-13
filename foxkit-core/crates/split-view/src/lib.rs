//! # Foxkit Split View
//!
//! Editor split panes and layout management.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

static PANE_ID: AtomicU64 = AtomicU64::new(1);

/// Split view service
pub struct SplitViewService {
    /// Root layout
    root: RwLock<LayoutNode>,
    /// Pane states
    panes: RwLock<HashMap<PaneId, PaneState>>,
    /// Active pane
    active_pane: RwLock<Option<PaneId>>,
    /// Configuration
    config: RwLock<SplitConfig>,
    /// Event sender
    event_tx: broadcast::Sender<SplitEvent>,
}

impl SplitViewService {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(64);
        
        // Create root with single pane
        let initial_pane = PaneId::new();
        let root = LayoutNode::Leaf(initial_pane.clone());

        let mut panes = HashMap::new();
        panes.insert(initial_pane.clone(), PaneState::new());

        Self {
            root: RwLock::new(root),
            panes: RwLock::new(panes),
            active_pane: RwLock::new(Some(initial_pane)),
            config: RwLock::new(SplitConfig::default()),
            event_tx,
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<SplitEvent> {
        self.event_tx.subscribe()
    }

    /// Split pane
    pub fn split(&self, pane_id: &PaneId, direction: SplitDirection) -> PaneId {
        let new_pane = PaneId::new();
        self.panes.write().insert(new_pane.clone(), PaneState::new());

        let mut root = self.root.write();
        *root = self.split_node(&root, pane_id, &new_pane, direction);

        let _ = self.event_tx.send(SplitEvent::Split {
            source: pane_id.clone(),
            new_pane: new_pane.clone(),
            direction,
        });

        new_pane
    }

    fn split_node(
        &self,
        node: &LayoutNode,
        target: &PaneId,
        new_pane: &PaneId,
        direction: SplitDirection,
    ) -> LayoutNode {
        match node {
            LayoutNode::Leaf(id) if id == target => {
                let orientation = match direction {
                    SplitDirection::Left | SplitDirection::Right => Orientation::Horizontal,
                    SplitDirection::Up | SplitDirection::Down => Orientation::Vertical,
                };

                let (first, second) = match direction {
                    SplitDirection::Left | SplitDirection::Up => {
                        (new_pane.clone(), target.clone())
                    }
                    SplitDirection::Right | SplitDirection::Down => {
                        (target.clone(), new_pane.clone())
                    }
                };

                LayoutNode::Split {
                    orientation,
                    ratio: 0.5,
                    first: Box::new(LayoutNode::Leaf(first)),
                    second: Box::new(LayoutNode::Leaf(second)),
                }
            }
            LayoutNode::Split { orientation, ratio, first, second } => {
                LayoutNode::Split {
                    orientation: *orientation,
                    ratio: *ratio,
                    first: Box::new(self.split_node(first, target, new_pane, direction)),
                    second: Box::new(self.split_node(second, target, new_pane, direction)),
                }
            }
            _ => node.clone(),
        }
    }

    /// Close pane
    pub fn close_pane(&self, pane_id: &PaneId) {
        self.panes.write().remove(pane_id);

        let mut root = self.root.write();
        if let Some(new_root) = self.remove_pane_from_node(&root, pane_id) {
            *root = new_root;
        }

        // Update active pane if necessary
        let mut active = self.active_pane.write();
        if active.as_ref() == Some(pane_id) {
            *active = self.panes.read().keys().next().cloned();
        }

        let _ = self.event_tx.send(SplitEvent::Closed(pane_id.clone()));
    }

    fn remove_pane_from_node(&self, node: &LayoutNode, target: &PaneId) -> Option<LayoutNode> {
        match node {
            LayoutNode::Leaf(id) if id == target => None,
            LayoutNode::Leaf(_) => Some(node.clone()),
            LayoutNode::Split { first, second, .. } => {
                let new_first = self.remove_pane_from_node(first, target);
                let new_second = self.remove_pane_from_node(second, target);

                match (new_first, new_second) {
                    (Some(f), Some(s)) => Some(LayoutNode::Split {
                        orientation: match node {
                            LayoutNode::Split { orientation, .. } => *orientation,
                            _ => Orientation::Horizontal,
                        },
                        ratio: match node {
                            LayoutNode::Split { ratio, .. } => *ratio,
                            _ => 0.5,
                        },
                        first: Box::new(f),
                        second: Box::new(s),
                    }),
                    (Some(f), None) => Some(f),
                    (None, Some(s)) => Some(s),
                    (None, None) => None,
                }
            }
        }
    }

    /// Set split ratio
    pub fn set_ratio(&self, pane_id: &PaneId, ratio: f32) {
        let ratio = ratio.clamp(0.1, 0.9);
        let mut root = self.root.write();
        self.set_ratio_in_node(&mut root, pane_id, ratio);
    }

    fn set_ratio_in_node(&self, node: &mut LayoutNode, target: &PaneId, new_ratio: f32) {
        if let LayoutNode::Split { ratio, first, second, .. } = node {
            if self.node_contains(first, target) || self.node_contains(second, target) {
                *ratio = new_ratio;
            }
            self.set_ratio_in_node(first, target, new_ratio);
            self.set_ratio_in_node(second, target, new_ratio);
        }
    }

    fn node_contains(&self, node: &LayoutNode, target: &PaneId) -> bool {
        match node {
            LayoutNode::Leaf(id) => id == target,
            LayoutNode::Split { first, second, .. } => {
                self.node_contains(first, target) || self.node_contains(second, target)
            }
        }
    }

    /// Focus pane
    pub fn focus(&self, pane_id: &PaneId) {
        if self.panes.read().contains_key(pane_id) {
            *self.active_pane.write() = Some(pane_id.clone());
            let _ = self.event_tx.send(SplitEvent::Focused(pane_id.clone()));
        }
    }

    /// Focus direction
    pub fn focus_direction(&self, direction: SplitDirection) {
        if let Some(active) = self.active_pane.read().clone() {
            if let Some(target) = self.find_pane_in_direction(&active, direction) {
                self.focus(&target);
            }
        }
    }

    fn find_pane_in_direction(&self, from: &PaneId, direction: SplitDirection) -> Option<PaneId> {
        let root = self.root.read();
        let all_panes = self.collect_panes(&root);
        
        // Simple implementation: cycle through panes
        let current_idx = all_panes.iter().position(|p| p == from)?;
        
        let next_idx = match direction {
            SplitDirection::Right | SplitDirection::Down => {
                (current_idx + 1) % all_panes.len()
            }
            SplitDirection::Left | SplitDirection::Up => {
                if current_idx == 0 {
                    all_panes.len() - 1
                } else {
                    current_idx - 1
                }
            }
        };

        all_panes.get(next_idx).cloned()
    }

    fn collect_panes(&self, node: &LayoutNode) -> Vec<PaneId> {
        match node {
            LayoutNode::Leaf(id) => vec![id.clone()],
            LayoutNode::Split { first, second, .. } => {
                let mut panes = self.collect_panes(first);
                panes.extend(self.collect_panes(second));
                panes
            }
        }
    }

    /// Maximize pane
    pub fn maximize(&self, pane_id: &PaneId) {
        if let Some(state) = self.panes.write().get_mut(pane_id) {
            state.maximized = true;
            let _ = self.event_tx.send(SplitEvent::Maximized(pane_id.clone()));
        }
    }

    /// Restore pane
    pub fn restore(&self, pane_id: &PaneId) {
        if let Some(state) = self.panes.write().get_mut(pane_id) {
            state.maximized = false;
            let _ = self.event_tx.send(SplitEvent::Restored(pane_id.clone()));
        }
    }

    /// Toggle maximize
    pub fn toggle_maximize(&self, pane_id: &PaneId) {
        if let Some(state) = self.panes.read().get(pane_id) {
            if state.maximized {
                self.restore(pane_id);
            } else {
                self.maximize(pane_id);
            }
        }
    }

    /// Get active pane
    pub fn active_pane(&self) -> Option<PaneId> {
        self.active_pane.read().clone()
    }

    /// Get layout
    pub fn layout(&self) -> LayoutNode {
        self.root.read().clone()
    }

    /// Get pane count
    pub fn pane_count(&self) -> usize {
        self.panes.read().len()
    }

    /// Get all pane IDs
    pub fn all_panes(&self) -> Vec<PaneId> {
        self.panes.read().keys().cloned().collect()
    }

    /// Reset to single pane
    pub fn reset(&self) {
        let new_pane = PaneId::new();
        
        *self.root.write() = LayoutNode::Leaf(new_pane.clone());
        
        let mut panes = self.panes.write();
        panes.clear();
        panes.insert(new_pane.clone(), PaneState::new());
        
        *self.active_pane.write() = Some(new_pane);
        
        let _ = self.event_tx.send(SplitEvent::Reset);
    }

    /// Compute layout rectangles
    pub fn compute_layout(&self, bounds: Rect) -> Vec<(PaneId, Rect)> {
        let root = self.root.read();
        self.compute_node_layout(&root, bounds)
    }

    fn compute_node_layout(&self, node: &LayoutNode, bounds: Rect) -> Vec<(PaneId, Rect)> {
        match node {
            LayoutNode::Leaf(id) => vec![(id.clone(), bounds)],
            LayoutNode::Split { orientation, ratio, first, second } => {
                let (first_bounds, second_bounds) = match orientation {
                    Orientation::Horizontal => {
                        let split = bounds.x + (bounds.width as f32 * ratio) as u32;
                        (
                            Rect { x: bounds.x, y: bounds.y, width: split - bounds.x, height: bounds.height },
                            Rect { x: split, y: bounds.y, width: bounds.width - (split - bounds.x), height: bounds.height },
                        )
                    }
                    Orientation::Vertical => {
                        let split = bounds.y + (bounds.height as f32 * ratio) as u32;
                        (
                            Rect { x: bounds.x, y: bounds.y, width: bounds.width, height: split - bounds.y },
                            Rect { x: bounds.x, y: split, width: bounds.width, height: bounds.height - (split - bounds.y) },
                        )
                    }
                };

                let mut layouts = self.compute_node_layout(first, first_bounds);
                layouts.extend(self.compute_node_layout(second, second_bounds));
                layouts
            }
        }
    }
}

impl Default for SplitViewService {
    fn default() -> Self {
        Self::new()
    }
}

/// Pane ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaneId(u64);

impl PaneId {
    fn new() -> Self {
        Self(PANE_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Pane state
#[derive(Debug, Clone, Default)]
pub struct PaneState {
    pub maximized: bool,
}

impl PaneState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Layout node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutNode {
    /// Single pane
    Leaf(PaneId),
    /// Split container
    Split {
        orientation: Orientation,
        ratio: f32,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

/// Split orientation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// Split direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SplitDirection {
    Left,
    Right,
    Up,
    Down,
}

/// Rectangle
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
}

/// Split event
#[derive(Debug, Clone)]
pub enum SplitEvent {
    Split {
        source: PaneId,
        new_pane: PaneId,
        direction: SplitDirection,
    },
    Closed(PaneId),
    Focused(PaneId),
    Maximized(PaneId),
    Restored(PaneId),
    Reset,
}

/// Split configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitConfig {
    /// Default split ratio
    pub default_ratio: f32,
    /// Minimum pane size
    pub min_pane_size: u32,
    /// Resize handle size
    pub handle_size: u32,
    /// Enable drag to resize
    pub enable_resize: bool,
}

impl Default for SplitConfig {
    fn default() -> Self {
        Self {
            default_ratio: 0.5,
            min_pane_size: 100,
            handle_size: 4,
            enable_resize: true,
        }
    }
}
