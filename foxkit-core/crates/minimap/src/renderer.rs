//! Minimap renderer

use crate::{Minimap, MinimapLine, MinimapPosition, MinimapRenderMode};
use serde::{Deserialize, Serialize};

/// Minimap configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimapConfig {
    /// Position
    pub position: MinimapPosition,
    /// Render mode
    pub render_mode: MinimapRenderMode,
    /// Width in pixels
    pub width: f32,
    /// Scale factor
    pub scale: f32,
    /// Show slider
    pub show_slider: bool,
    /// Slider color
    pub slider_color: String,
    /// Max columns to render
    pub max_columns: usize,
    /// Render characters or just blocks
    pub render_characters: bool,
}

impl Default for MinimapConfig {
    fn default() -> Self {
        Self {
            position: MinimapPosition::Right,
            render_mode: MinimapRenderMode::Blocks,
            width: 120.0,
            scale: 1.0,
            show_slider: true,
            slider_color: "rgba(100, 100, 100, 0.5)".to_string(),
            max_columns: 120,
            render_characters: false,
        }
    }
}

/// Minimap renderer
pub struct MinimapRenderer {
    config: MinimapConfig,
}

impl MinimapRenderer {
    pub fn new(config: MinimapConfig) -> Self {
        Self { config }
    }

    /// Render minimap to canvas commands
    pub fn render(&self, minimap: &Minimap, height: f32) -> Vec<RenderCommand> {
        let mut commands = Vec::new();

        if minimap.total_lines == 0 {
            return commands;
        }

        let line_height = height / minimap.total_lines as f32;
        let char_width = self.config.width / self.config.max_columns as f32;

        // Render lines
        for line in &minimap.lines {
            let y = line.index as f32 * line_height;
            
            if line.is_blank {
                continue;
            }

            let visible_length = line.length.min(self.config.max_columns);
            let x_start = line.indent as f32 * char_width;
            let width = (visible_length - line.indent) as f32 * char_width;

            match self.config.render_mode {
                MinimapRenderMode::Characters => {
                    commands.push(RenderCommand::Text {
                        x: 0.0,
                        y,
                        height: line_height,
                        line_index: line.index,
                    });
                }
                MinimapRenderMode::Blocks => {
                    commands.push(RenderCommand::Rect {
                        x: x_start,
                        y,
                        width,
                        height: line_height.max(1.0),
                        color: self.line_color(line),
                    });
                }
                MinimapRenderMode::Dots => {
                    for i in 0..(visible_length - line.indent) {
                        commands.push(RenderCommand::Dot {
                            x: x_start + i as f32 * char_width,
                            y,
                            radius: char_width / 2.0,
                            color: self.line_color(line),
                        });
                    }
                }
            }
        }

        // Render highlights
        for highlight in &minimap.highlights {
            let y = highlight.line as f32 * line_height;
            let h = ((highlight.end_line - highlight.line + 1) as f32 * line_height).max(1.0);
            
            commands.push(RenderCommand::Highlight {
                x: 0.0,
                y,
                width: self.config.width,
                height: h,
                color: highlight.color.clone(),
            });
        }

        // Render visible region slider
        if self.config.show_slider {
            let (start_y, end_y) = minimap.visible_region_bounds(height);
            commands.push(RenderCommand::Slider {
                x: 0.0,
                y: start_y,
                width: self.config.width,
                height: (end_y - start_y).max(10.0),
                color: self.config.slider_color.clone(),
            });
        }

        commands
    }

    fn line_color(&self, line: &MinimapLine) -> String {
        if !line.tokens.is_empty() {
            // Use first token's color
            return self.scope_to_color(&line.tokens[0].scope);
        }
        
        // Default text color
        "rgba(200, 200, 200, 0.7)".to_string()
    }

    fn scope_to_color(&self, scope: &str) -> String {
        match scope {
            "keyword" => "rgba(198, 120, 221, 0.8)".to_string(),
            "string" => "rgba(152, 195, 121, 0.8)".to_string(),
            "comment" => "rgba(92, 99, 112, 0.6)".to_string(),
            "number" => "rgba(209, 154, 102, 0.8)".to_string(),
            "function" => "rgba(97, 175, 239, 0.8)".to_string(),
            "type" => "rgba(229, 192, 123, 0.8)".to_string(),
            _ => "rgba(171, 178, 191, 0.7)".to_string(),
        }
    }

    /// Get config
    pub fn config(&self) -> &MinimapConfig {
        &self.config
    }

    /// Set config
    pub fn set_config(&mut self, config: MinimapConfig) {
        self.config = config;
    }
}

impl Default for MinimapRenderer {
    fn default() -> Self {
        Self::new(MinimapConfig::default())
    }
}

/// Render command for minimap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RenderCommand {
    /// Rectangle
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: String,
    },
    /// Dot
    Dot {
        x: f32,
        y: f32,
        radius: f32,
        color: String,
    },
    /// Text line
    Text {
        x: f32,
        y: f32,
        height: f32,
        line_index: usize,
    },
    /// Highlight region
    Highlight {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: String,
    },
    /// Visible region slider
    Slider {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Minimap;

    #[test]
    fn test_render() {
        let mut minimap = Minimap::new(0, MinimapConfig::default());
        minimap.update_from_content("fn main() {\n    println!(\"Hello\");\n}\n");
        minimap.set_visible_range(0, 3);

        let renderer = MinimapRenderer::default();
        let commands = renderer.render(&minimap, 100.0);
        
        assert!(!commands.is_empty());
    }
}
