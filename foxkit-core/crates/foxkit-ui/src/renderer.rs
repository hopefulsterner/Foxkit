//! Foxkit UI Renderer
//!
//! Bridges the UI component system with the GPU renderer.
//! Converts AppView into render primitives that can be sent to the GPU.

use crate::theme::Theme;
use crate::app::{AppView, AppLayout, Position};
use crate::components::activity_bar::{ActivityBarView, Color};
use crate::components::sidebar::SidebarView;
use crate::components::editor_area::EditorAreaView;
use crate::components::bottom_panel::{BottomPanelView, PanelContent};
use crate::components::status_bar::StatusBarView;

/// Convert our Color to f32 RGBA array
pub fn color_to_rgba(color: Color) -> [f32; 4] {
    [color.r, color.g, color.b, color.a]
}

/// Layout bounds for each component
#[derive(Debug, Clone, Copy)]
pub struct LayoutBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl LayoutBounds {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }
}

/// Calculated bounds for all UI components
#[derive(Debug, Clone)]
pub struct ComputedLayout {
    pub activity_bar: LayoutBounds,
    pub sidebar: LayoutBounds,
    pub editor: LayoutBounds,
    pub bottom_panel: LayoutBounds,
    pub status_bar: LayoutBounds,
}

impl ComputedLayout {
    /// Calculate layout bounds from app layout and window size
    pub fn compute(layout: &AppLayout, theme: &Theme, window_width: f32, window_height: f32) -> Self {
        let activity_bar_width = if layout.show_activity_bar {
            theme.spacing.activity_bar_width
        } else {
            0.0
        };

        let status_bar_height = if layout.show_status_bar {
            theme.spacing.status_bar_height
        } else {
            0.0
        };

        let sidebar_width = if layout.show_sidebar {
            layout.sidebar_width
        } else {
            0.0
        };

        let bottom_panel_height = if layout.show_bottom_panel {
            layout.bottom_panel_height
        } else {
            0.0
        };

        let (activity_bar_x, sidebar_x, editor_x) = match (layout.activity_bar_position, layout.sidebar_position) {
            (Position::Left, Position::Left) => {
                (0.0, activity_bar_width, activity_bar_width + sidebar_width)
            }
            (Position::Left, Position::Right) => {
                (0.0, window_width - sidebar_width, activity_bar_width)
            }
            (Position::Right, Position::Left) => {
                (window_width - activity_bar_width, 0.0, sidebar_width)
            }
            (Position::Right, Position::Right) => {
                (window_width - activity_bar_width, window_width - activity_bar_width - sidebar_width, 0.0)
            }
        };

        let editor_width = window_width - activity_bar_width - sidebar_width;
        let editor_height = window_height - status_bar_height - bottom_panel_height;

        Self {
            activity_bar: LayoutBounds::new(
                activity_bar_x,
                0.0,
                activity_bar_width,
                window_height - status_bar_height,
            ),
            sidebar: LayoutBounds::new(
                sidebar_x,
                0.0,
                sidebar_width,
                window_height - status_bar_height - bottom_panel_height,
            ),
            editor: LayoutBounds::new(
                editor_x,
                0.0,
                editor_width,
                editor_height,
            ),
            bottom_panel: LayoutBounds::new(
                if layout.sidebar_position == Position::Left { 
                    activity_bar_width + sidebar_width 
                } else { 
                    0.0 
                },
                editor_height,
                editor_width,
                bottom_panel_height,
            ),
            status_bar: LayoutBounds::new(
                0.0,
                window_height - status_bar_height,
                window_width,
                status_bar_height,
            ),
        }
    }
}

/// UI Scene - collection of primitives to render
#[derive(Default)]
pub struct UiScene {
    pub rectangles: Vec<RectPrimitive>,
    pub texts: Vec<TextPrimitive>,
    pub icons: Vec<IconPrimitive>,
}

impl UiScene {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Rectangle primitive for rendering
#[derive(Debug, Clone)]
pub struct RectPrimitive {
    pub bounds: LayoutBounds,
    pub color: [f32; 4],
    pub border_color: Option<[f32; 4]>,
    pub border_width: f32,
    pub corner_radius: f32,
}

/// Text primitive for rendering  
#[derive(Debug, Clone)]
pub struct TextPrimitive {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub color: [f32; 4],
    pub font_size: f32,
    pub font_family: FontFamily,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFamily {
    Ui,
    Editor,
    Terminal,
}

/// Icon primitive for rendering
#[derive(Debug, Clone)]
pub struct IconPrimitive {
    pub x: f32,
    pub y: f32,
    pub icon: String,
    pub color: [f32; 4],
    pub size: f32,
}

/// UI Renderer - converts AppView into render primitives
pub struct UiRenderer {
    pub theme: Theme,
}

impl UiRenderer {
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }

    /// Render the full application view
    pub fn render(&self, view: &AppView, window_width: f32, window_height: f32) -> UiScene {
        let mut scene = UiScene::new();
        let layout = ComputedLayout::compute(view.layout, view.theme, window_width, window_height);

        self.render_activity_bar(&mut scene, &view.activity_bar, &layout.activity_bar);

        if view.layout.show_sidebar {
            self.render_sidebar(&mut scene, &view.sidebar, &layout.sidebar);
        }

        self.render_editor_area(&mut scene, &view.editor_area, &layout.editor);

        if view.layout.show_bottom_panel {
            self.render_bottom_panel(&mut scene, &view.bottom_panel, &layout.bottom_panel);
        }

        if view.layout.show_status_bar {
            self.render_status_bar(&mut scene, &view.status_bar, &layout.status_bar);
        }

        scene
    }

    fn render_activity_bar(&self, scene: &mut UiScene, view: &ActivityBarView, bounds: &LayoutBounds) {
        scene.rectangles.push(RectPrimitive {
            bounds: *bounds,
            color: color_to_rgba(view.colors.background),
            border_color: None,
            border_width: 0.0,
            corner_radius: 0.0,
        });

        let item_height = 48.0;
        for (i, item) in view.items.iter().enumerate() {
            let y = bounds.y + (i as f32 * item_height) + 8.0;

            if item.is_active {
                scene.rectangles.push(RectPrimitive {
                    bounds: LayoutBounds::new(bounds.x, y - 4.0, bounds.width, 40.0),
                    color: color_to_rgba(view.colors.active_background),
                    border_color: None,
                    border_width: 0.0,
                    corner_radius: 0.0,
                });
                
                scene.rectangles.push(RectPrimitive {
                    bounds: LayoutBounds::new(bounds.x, y - 4.0, 2.0, 40.0),
                    color: color_to_rgba(view.colors.active_border),
                    border_color: None,
                    border_width: 0.0,
                    corner_radius: 0.0,
                });
            }

            scene.icons.push(IconPrimitive {
                x: bounds.x + 14.0,
                y: y + 6.0,
                icon: item.icon.to_string(),
                color: if item.is_active {
                    color_to_rgba(view.colors.foreground)
                } else {
                    color_to_rgba(view.colors.inactive_foreground)
                },
                size: 20.0,
            });

            if let Some(ref badge) = item.badge {
                let badge_x = bounds.x + 30.0;
                let badge_y = y + 2.0;
                
                scene.rectangles.push(RectPrimitive {
                    bounds: LayoutBounds::new(badge_x, badge_y, 16.0, 16.0),
                    color: color_to_rgba(view.colors.badge_background),
                    border_color: None,
                    border_width: 0.0,
                    corner_radius: 8.0,
                });
                
                scene.texts.push(TextPrimitive {
                    x: badge_x + 4.0,
                    y: badge_y + 2.0,
                    text: badge.clone(),
                    color: color_to_rgba(view.colors.badge_foreground),
                    font_size: 10.0,
                    font_family: FontFamily::Ui,
                });
            }
        }
    }

    fn render_sidebar(&self, scene: &mut UiScene, view: &SidebarView, bounds: &LayoutBounds) {
        // Background
        scene.rectangles.push(RectPrimitive {
            bounds: *bounds,
            color: color_to_rgba(view.background),
            border_color: Some(color_to_rgba(view.border_color)),
            border_width: 1.0,
            corner_radius: 0.0,
        });

        // Header
        let header_height = 35.0;
        scene.rectangles.push(RectPrimitive {
            bounds: LayoutBounds::new(bounds.x, bounds.y, bounds.width, header_height),
            color: color_to_rgba(view.header_bg),
            border_color: None,
            border_width: 0.0,
            corner_radius: 0.0,
        });

        scene.texts.push(TextPrimitive {
            x: bounds.x + 12.0,
            y: bounds.y + 10.0,
            text: view.title.to_string(),
            color: color_to_rgba(view.header_fg),
            font_size: 11.0,
            font_family: FontFamily::Ui,
        });

        // Content is rendered based on SidebarContent variant
        // For now, just draw a placeholder - full implementation pending
    }

    fn render_editor_area(&self, scene: &mut UiScene, view: &EditorAreaView, bounds: &LayoutBounds) {
        // Editor area background
        scene.rectangles.push(RectPrimitive {
            bounds: *bounds,
            color: color_to_rgba(view.colors.background),
            border_color: None,
            border_width: 0.0,
            corner_radius: 0.0,
        });

        // For now just render the first group
        if let Some(group) = view.groups.first() {
            let tab_bar = &group.tabs;
            let tab_height = 35.0;
            
            // Tab bar background
            scene.rectangles.push(RectPrimitive {
                bounds: LayoutBounds::new(bounds.x, bounds.y, bounds.width, tab_height),
                color: color_to_rgba(tab_bar.colors.background),
                border_color: None,
                border_width: 0.0,
                corner_radius: 0.0,
            });

            // Render tabs
            let tab_width = 150.0;
            let mut tab_x = bounds.x;
            
            for tab in &tab_bar.tabs {
                let tab_bg = if tab.is_active {
                    tab_bar.colors.active_bg
                } else {
                    tab_bar.colors.inactive_bg
                };
                
                scene.rectangles.push(RectPrimitive {
                    bounds: LayoutBounds::new(tab_x, bounds.y, tab_width, tab_height),
                    color: color_to_rgba(tab_bg),
                    border_color: None,
                    border_width: 0.0,
                    corner_radius: 0.0,
                });

                let text_color = if tab.is_active {
                    tab_bar.colors.active_fg
                } else {
                    tab_bar.colors.inactive_fg
                };

                scene.texts.push(TextPrimitive {
                    x: tab_x + 12.0,
                    y: bounds.y + 10.0,
                    text: tab.title.clone(),
                    color: color_to_rgba(text_color),
                    font_size: 13.0,
                    font_family: FontFamily::Ui,
                });

                if tab.modified {
                    scene.rectangles.push(RectPrimitive {
                        bounds: LayoutBounds::new(tab_x + tab_width - 16.0, bounds.y + 14.0, 8.0, 8.0),
                        color: color_to_rgba(tab_bar.colors.modified_dot),
                        border_color: None,
                        border_width: 0.0,
                        corner_radius: 4.0,
                    });
                }

                tab_x += tab_width;
            }

            // Editor content area
            let content_y = bounds.y + tab_height;
            let content_height = bounds.height - tab_height;
            
            scene.rectangles.push(RectPrimitive {
                bounds: LayoutBounds::new(bounds.x, content_y, bounds.width, content_height),
                color: color_to_rgba(view.colors.background),
                border_color: None,
                border_width: 0.0,
                corner_radius: 0.0,
            });

            // Gutter
            let gutter_width = 50.0;
            scene.rectangles.push(RectPrimitive {
                bounds: LayoutBounds::new(bounds.x, content_y, gutter_width, content_height),
                color: color_to_rgba(view.colors.gutter_bg),
                border_color: None,
                border_width: 0.0,
                corner_radius: 0.0,
            });
        }
    }

    fn render_bottom_panel(&self, scene: &mut UiScene, view: &BottomPanelView, bounds: &LayoutBounds) {
        // Background
        scene.rectangles.push(RectPrimitive {
            bounds: *bounds,
            color: color_to_rgba(view.colors.background),
            border_color: None,
            border_width: 0.0,
            corner_radius: 0.0,
        });

        // Top border
        scene.rectangles.push(RectPrimitive {
            bounds: LayoutBounds::new(bounds.x, bounds.y, bounds.width, 1.0),
            color: color_to_rgba(view.colors.border),
            border_color: None,
            border_width: 0.0,
            corner_radius: 0.0,
        });

        // Header/tab bar
        let header_height = 35.0;
        scene.rectangles.push(RectPrimitive {
            bounds: LayoutBounds::new(bounds.x, bounds.y, bounds.width, header_height),
            color: color_to_rgba(view.colors.header_bg),
            border_color: None,
            border_width: 0.0,
            corner_radius: 0.0,
        });

        // Render tabs from TabBarView
        let tab_width = 100.0;
        let mut tab_x = bounds.x;
        let tab_bar = &view.tabs;
        
        for tab in &tab_bar.tabs {
            let text_color = if tab.is_active {
                tab_bar.colors.active_fg
            } else {
                tab_bar.colors.inactive_fg
            };
            
            scene.texts.push(TextPrimitive {
                x: tab_x + 12.0,
                y: bounds.y + 10.0,
                text: tab.title.clone(),
                color: color_to_rgba(text_color),
                font_size: 12.0,
                font_family: FontFamily::Ui,
            });

            tab_x += tab_width;
        }

        // Content area
        let content_y = bounds.y + header_height;
        let content_height = bounds.height - header_height;
        let content_bounds = LayoutBounds::new(bounds.x, content_y, bounds.width, content_height);

        match &view.content {
            PanelContent::Terminal(term) => {
                self.render_terminal_content(scene, term, &content_bounds);
            }
            PanelContent::Problems(problems) => {
                self.render_problems_content(scene, problems, &content_bounds);
            }
            PanelContent::Output(output) => {
                self.render_output_content(scene, output, &content_bounds);
            }
            PanelContent::DebugConsole(debug) => {
                self.render_debug_content(scene, debug, &content_bounds);
            }
            PanelContent::AiChat(chat) => {
                self.render_chat_content(scene, chat, &content_bounds);
            }
            PanelContent::Empty => {
                // Empty panel - nothing to render
            }
        }
    }

    fn render_terminal_content(&self, scene: &mut UiScene, view: &crate::components::bottom_panel::TerminalView, bounds: &LayoutBounds) {
        scene.rectangles.push(RectPrimitive {
            bounds: *bounds,
            color: color_to_rgba(view.colors.background),
            border_color: None,
            border_width: 0.0,
            corner_radius: 0.0,
        });

        let line_height = 18.0;
        for (i, line) in view.lines.iter().enumerate() {
            scene.texts.push(TextPrimitive {
                x: bounds.x + 8.0,
                y: bounds.y + (i as f32 * line_height) + 4.0,
                text: line.text.clone(),
                color: color_to_rgba(view.colors.foreground),
                font_size: 13.0,
                font_family: FontFamily::Terminal,
            });
        }

        let char_width = 8.0;
        let cursor_x = bounds.x + 8.0 + (view.cursor_x as f32 * char_width);
        let cursor_y = bounds.y + (view.cursor_y as f32 * line_height) + 4.0;
        
        scene.rectangles.push(RectPrimitive {
            bounds: LayoutBounds::new(cursor_x, cursor_y, char_width, line_height),
            color: color_to_rgba(view.colors.cursor),
            border_color: None,
            border_width: 0.0,
            corner_radius: 0.0,
        });
    }

    fn render_problems_content(&self, scene: &mut UiScene, view: &crate::components::bottom_panel::ProblemsView, bounds: &LayoutBounds) {
        use crate::components::bottom_panel::ProblemSeverity;
        
        let row_height = 22.0;
        let default_fg = Color::hex(0xCCCCCC);
        
        for (i, item) in view.items.iter().enumerate() {
            let y = bounds.y + (i as f32 * row_height) + 4.0;
            
            let (icon_color, icon_name) = match item.severity {
                ProblemSeverity::Error => ([1.0, 0.3, 0.3, 1.0], "error"),
                ProblemSeverity::Warning => ([1.0, 0.8, 0.0, 1.0], "warning"),
                ProblemSeverity::Info => ([0.3, 0.7, 1.0, 1.0], "info"),
                ProblemSeverity::Hint => ([0.7, 0.7, 0.7, 1.0], "lightbulb"),
            };

            scene.icons.push(IconPrimitive {
                x: bounds.x + 8.0,
                y,
                icon: icon_name.to_string(),
                color: icon_color,
                size: 16.0,
            });

            scene.texts.push(TextPrimitive {
                x: bounds.x + 28.0,
                y,
                text: item.message.clone(),
                color: color_to_rgba(default_fg),
                font_size: 12.0,
                font_family: FontFamily::Ui,
            });

            scene.texts.push(TextPrimitive {
                x: bounds.x + 400.0,
                y,
                text: format!("{} [{}:{}]", item.file, item.line, item.column),
                color: color_to_rgba(Color::hex(0x808080)),
                font_size: 12.0,
                font_family: FontFamily::Ui,
            });
        }
    }

    fn render_output_content(&self, scene: &mut UiScene, view: &crate::components::bottom_panel::OutputView, bounds: &LayoutBounds) {
        let line_height = 18.0;
        let default_fg = Color::hex(0xCCCCCC);
        
        for (i, line) in view.lines.iter().enumerate() {
            scene.texts.push(TextPrimitive {
                x: bounds.x + 8.0,
                y: bounds.y + (i as f32 * line_height) + 4.0,
                text: line.clone(),
                color: color_to_rgba(default_fg),
                font_size: 12.0,
                font_family: FontFamily::Terminal,
            });
        }
    }

    fn render_debug_content(&self, scene: &mut UiScene, view: &crate::components::bottom_panel::DebugConsoleView, bounds: &LayoutBounds) {
        let line_height = 20.0;
        
        for (i, entry) in view.entries.iter().enumerate() {
            let y = bounds.y + (i as f32 * line_height) + 4.0;
            
            let text_color = match entry.kind {
                crate::components::bottom_panel::DebugConsoleEntryKind::Input => Color::hex(0x569CD6),
                crate::components::bottom_panel::DebugConsoleEntryKind::Output => Color::hex(0xCCCCCC),
                crate::components::bottom_panel::DebugConsoleEntryKind::Error => Color::hex(0xF48771),
                crate::components::bottom_panel::DebugConsoleEntryKind::Info => Color::hex(0x808080),
            };

            scene.texts.push(TextPrimitive {
                x: bounds.x + 8.0,
                y,
                text: entry.message.clone(),
                color: color_to_rgba(text_color),
                font_size: 12.0,
                font_family: FontFamily::Terminal,
            });
        }
    }

    fn render_chat_content(&self, scene: &mut UiScene, view: &crate::components::bottom_panel::AiChatView, bounds: &LayoutBounds) {
        let mut y = bounds.y + 8.0;
        let bubble_width = 300.0;
        
        for msg in &view.messages {
            let is_user = msg.role == crate::components::bottom_panel::ChatRole::User;
            let bubble_color = if is_user {
                color_to_rgba(view.colors.user_bubble_bg)
            } else {
                color_to_rgba(view.colors.assistant_bubble_bg)
            };

            let lines = (msg.content.len() / 50).max(1);
            let msg_height = lines as f32 * 20.0 + 16.0;

            let bubble_x = if is_user {
                bounds.x + bounds.width - bubble_width - 16.0
            } else {
                bounds.x + 16.0
            };

            scene.rectangles.push(RectPrimitive {
                bounds: LayoutBounds::new(bubble_x, y, bubble_width, msg_height),
                color: bubble_color,
                border_color: None,
                border_width: 0.0,
                corner_radius: 8.0,
            });

            scene.texts.push(TextPrimitive {
                x: bubble_x + 12.0,
                y: y + 10.0,
                text: msg.content.clone(),
                color: color_to_rgba(view.colors.foreground),
                font_size: 13.0,
                font_family: FontFamily::Ui,
            });

            y += msg_height + 12.0;
        }

        let input_height = 50.0;
        let input_y = bounds.y + bounds.height - input_height;
        
        scene.rectangles.push(RectPrimitive {
            bounds: LayoutBounds::new(bounds.x, input_y, bounds.width, input_height),
            color: color_to_rgba(view.colors.input_bg),
            border_color: Some(color_to_rgba(view.colors.input_border)),
            border_width: 1.0,
            corner_radius: 0.0,
        });

        let display_text = if view.input.is_empty() {
            "Ask Foxkit AI...".to_string()
        } else {
            view.input.clone()
        };
        
        let text_color = if view.input.is_empty() {
            color_to_rgba(view.colors.placeholder_foreground)
        } else {
            color_to_rgba(view.colors.foreground)
        };

        scene.texts.push(TextPrimitive {
            x: bounds.x + 12.0,
            y: input_y + 16.0,
            text: display_text,
            color: text_color,
            font_size: 14.0,
            font_family: FontFamily::Ui,
        });
    }

    fn render_status_bar(&self, scene: &mut UiScene, view: &StatusBarView, bounds: &LayoutBounds) {
        scene.rectangles.push(RectPrimitive {
            bounds: *bounds,
            color: color_to_rgba(view.colors.background),
            border_color: None,
            border_width: 0.0,
            corner_radius: 0.0,
        });

        let mut left_x = bounds.x;
        if let Some(ref remote) = view.remote {
            let remote_width = 100.0;
            scene.rectangles.push(RectPrimitive {
                bounds: LayoutBounds::new(left_x, bounds.y, remote_width, bounds.height),
                color: color_to_rgba(view.colors.remote_bg),
                border_color: None,
                border_width: 0.0,
                corner_radius: 0.0,
            });

            scene.icons.push(IconPrimitive {
                x: left_x + 8.0,
                y: bounds.y + 3.0,
                icon: remote.icon.to_string(),
                color: color_to_rgba(view.colors.foreground),
                size: 14.0,
            });

            scene.texts.push(TextPrimitive {
                x: left_x + 26.0,
                y: bounds.y + 4.0,
                text: remote.name.clone(),
                color: color_to_rgba(view.colors.foreground),
                font_size: 12.0,
                font_family: FontFamily::Ui,
            });

            left_x += remote_width;
        }

        for item in &view.left_items {
            if let Some(icon) = &item.icon {
                scene.icons.push(IconPrimitive {
                    x: left_x + 8.0,
                    y: bounds.y + 3.0,
                    icon: icon.to_string(),
                    color: color_to_rgba(view.colors.foreground),
                    size: 14.0,
                });
                left_x += 20.0;
            }

            if !item.text.is_empty() {
                scene.texts.push(TextPrimitive {
                    x: left_x + 4.0,
                    y: bounds.y + 4.0,
                    text: item.text.clone(),
                    color: color_to_rgba(view.colors.foreground),
                    font_size: 12.0,
                    font_family: FontFamily::Ui,
                });
                left_x += item.text.len() as f32 * 7.0 + 16.0;
            }
        }

        let mut right_x = bounds.x + bounds.width - 16.0;
        for item in view.right_items.iter().rev() {
            if !item.text.is_empty() {
                let text_width = item.text.len() as f32 * 7.0;
                right_x -= text_width;
                scene.texts.push(TextPrimitive {
                    x: right_x,
                    y: bounds.y + 4.0,
                    text: item.text.clone(),
                    color: color_to_rgba(view.colors.foreground),
                    font_size: 12.0,
                    font_family: FontFamily::Ui,
                });
            }

            if let Some(icon) = &item.icon {
                right_x -= 20.0;
                scene.icons.push(IconPrimitive {
                    x: right_x,
                    y: bounds.y + 3.0,
                    icon: icon.to_string(),
                    color: color_to_rgba(view.colors.foreground),
                    size: 14.0,
                });
            }

            right_x -= 12.0;
        }
    }
}
