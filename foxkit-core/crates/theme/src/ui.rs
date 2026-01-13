//! UI theme colors

use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::ThemeKind;

/// UI theme colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiTheme {
    // Base
    pub background: Color,
    pub foreground: Color,
    pub border: Color,

    // Editor
    pub editor_background: Color,
    pub editor_foreground: Color,
    pub editor_line_number: Color,
    pub editor_line_number_active: Color,
    pub editor_selection: Color,
    pub editor_cursor: Color,
    pub editor_whitespace: Color,
    pub editor_indent_guide: Color,

    // Activity bar
    pub activity_bar_background: Color,
    pub activity_bar_foreground: Color,
    pub activity_bar_badge: Color,
    pub activity_bar_badge_foreground: Color,

    // Side bar
    pub sidebar_background: Color,
    pub sidebar_foreground: Color,
    pub sidebar_section_header: Color,

    // Title bar
    pub title_bar_background: Color,
    pub title_bar_foreground: Color,

    // Tabs
    pub tab_active_background: Color,
    pub tab_active_foreground: Color,
    pub tab_inactive_background: Color,
    pub tab_inactive_foreground: Color,
    pub tab_border: Color,

    // Status bar
    pub status_bar_background: Color,
    pub status_bar_foreground: Color,
    pub status_bar_debugging: Color,
    pub status_bar_no_folder: Color,

    // Panel
    pub panel_background: Color,
    pub panel_foreground: Color,
    pub panel_border: Color,

    // Terminal
    pub terminal_background: Color,
    pub terminal_foreground: Color,
    pub terminal_cursor: Color,
    pub terminal_selection: Color,
    
    // ANSI colors
    pub terminal_black: Color,
    pub terminal_red: Color,
    pub terminal_green: Color,
    pub terminal_yellow: Color,
    pub terminal_blue: Color,
    pub terminal_magenta: Color,
    pub terminal_cyan: Color,
    pub terminal_white: Color,
    pub terminal_bright_black: Color,
    pub terminal_bright_red: Color,
    pub terminal_bright_green: Color,
    pub terminal_bright_yellow: Color,
    pub terminal_bright_blue: Color,
    pub terminal_bright_magenta: Color,
    pub terminal_bright_cyan: Color,
    pub terminal_bright_white: Color,

    // Button
    pub button_background: Color,
    pub button_foreground: Color,
    pub button_hover: Color,
    pub button_secondary_background: Color,
    pub button_secondary_foreground: Color,

    // Input
    pub input_background: Color,
    pub input_foreground: Color,
    pub input_border: Color,
    pub input_placeholder: Color,

    // List
    pub list_active_background: Color,
    pub list_active_foreground: Color,
    pub list_hover_background: Color,
    pub list_focus_background: Color,

    // Accent
    pub accent: Color,
    pub accent_foreground: Color,
    pub link: Color,
    pub error: Color,
    pub warning: Color,
    pub info: Color,
    pub success: Color,

    // Git
    pub git_added: Color,
    pub git_modified: Color,
    pub git_deleted: Color,
    pub git_untracked: Color,
    pub git_ignored: Color,
    pub git_conflict: Color,
}

impl UiTheme {
    /// Default dark UI theme
    pub fn dark() -> Self {
        Self {
            // Base
            background: Color::hex(0x1E1E1E),
            foreground: Color::hex(0xCCCCCC),
            border: Color::hex(0x454545),

            // Editor
            editor_background: Color::hex(0x1E1E1E),
            editor_foreground: Color::hex(0xD4D4D4),
            editor_line_number: Color::hex(0x858585),
            editor_line_number_active: Color::hex(0xC6C6C6),
            editor_selection: Color::rgba(38, 79, 120, 180),
            editor_cursor: Color::hex(0xAEAFAD),
            editor_whitespace: Color::hex(0x3B3B3B),
            editor_indent_guide: Color::hex(0x404040),

            // Activity bar
            activity_bar_background: Color::hex(0x333333),
            activity_bar_foreground: Color::hex(0xFFFFFF),
            activity_bar_badge: Color::hex(0x007ACC),
            activity_bar_badge_foreground: Color::hex(0xFFFFFF),

            // Side bar
            sidebar_background: Color::hex(0x252526),
            sidebar_foreground: Color::hex(0xCCCCCC),
            sidebar_section_header: Color::hex(0x383838),

            // Title bar
            title_bar_background: Color::hex(0x3C3C3C),
            title_bar_foreground: Color::hex(0xCCCCCC),

            // Tabs
            tab_active_background: Color::hex(0x1E1E1E),
            tab_active_foreground: Color::hex(0xFFFFFF),
            tab_inactive_background: Color::hex(0x2D2D2D),
            tab_inactive_foreground: Color::hex(0x969696),
            tab_border: Color::hex(0x252526),

            // Status bar
            status_bar_background: Color::hex(0x007ACC),
            status_bar_foreground: Color::hex(0xFFFFFF),
            status_bar_debugging: Color::hex(0xCC6633),
            status_bar_no_folder: Color::hex(0x68217A),

            // Panel
            panel_background: Color::hex(0x1E1E1E),
            panel_foreground: Color::hex(0xCCCCCC),
            panel_border: Color::hex(0x454545),

            // Terminal
            terminal_background: Color::hex(0x1E1E1E),
            terminal_foreground: Color::hex(0xCCCCCC),
            terminal_cursor: Color::hex(0xFFFFFF),
            terminal_selection: Color::rgba(255, 255, 255, 50),
            
            // ANSI colors
            terminal_black: Color::hex(0x000000),
            terminal_red: Color::hex(0xCD3131),
            terminal_green: Color::hex(0x0DBC79),
            terminal_yellow: Color::hex(0xE5E510),
            terminal_blue: Color::hex(0x2472C8),
            terminal_magenta: Color::hex(0xBC3FBC),
            terminal_cyan: Color::hex(0x11A8CD),
            terminal_white: Color::hex(0xE5E5E5),
            terminal_bright_black: Color::hex(0x666666),
            terminal_bright_red: Color::hex(0xF14C4C),
            terminal_bright_green: Color::hex(0x23D18B),
            terminal_bright_yellow: Color::hex(0xF5F543),
            terminal_bright_blue: Color::hex(0x3B8EEA),
            terminal_bright_magenta: Color::hex(0xD670D6),
            terminal_bright_cyan: Color::hex(0x29B8DB),
            terminal_bright_white: Color::hex(0xFFFFFF),

            // Button
            button_background: Color::hex(0x0E639C),
            button_foreground: Color::hex(0xFFFFFF),
            button_hover: Color::hex(0x1177BB),
            button_secondary_background: Color::hex(0x3A3D41),
            button_secondary_foreground: Color::hex(0xFFFFFF),

            // Input
            input_background: Color::hex(0x3C3C3C),
            input_foreground: Color::hex(0xCCCCCC),
            input_border: Color::hex(0x3C3C3C),
            input_placeholder: Color::hex(0xA6A6A6),

            // List
            list_active_background: Color::hex(0x094771),
            list_active_foreground: Color::hex(0xFFFFFF),
            list_hover_background: Color::hex(0x2A2D2E),
            list_focus_background: Color::hex(0x062F4A),

            // Accent
            accent: Color::hex(0x007ACC),
            accent_foreground: Color::hex(0xFFFFFF),
            link: Color::hex(0x3794FF),
            error: Color::hex(0xF48771),
            warning: Color::hex(0xCCA700),
            info: Color::hex(0x75BEFF),
            success: Color::hex(0x89D185),

            // Git
            git_added: Color::hex(0x81B88B),
            git_modified: Color::hex(0xE2C08D),
            git_deleted: Color::hex(0xC74E39),
            git_untracked: Color::hex(0x73C991),
            git_ignored: Color::hex(0x8C8C8C),
            git_conflict: Color::hex(0xE4676B),
        }
    }

    /// Default light UI theme
    pub fn light() -> Self {
        Self {
            // Base
            background: Color::hex(0xFFFFFF),
            foreground: Color::hex(0x333333),
            border: Color::hex(0xE5E5E5),

            // Editor
            editor_background: Color::hex(0xFFFFFF),
            editor_foreground: Color::hex(0x000000),
            editor_line_number: Color::hex(0x237893),
            editor_line_number_active: Color::hex(0x0B216F),
            editor_selection: Color::rgba(173, 214, 255, 180),
            editor_cursor: Color::hex(0x000000),
            editor_whitespace: Color::hex(0xD3D3D3),
            editor_indent_guide: Color::hex(0xD3D3D3),

            // Activity bar
            activity_bar_background: Color::hex(0x2C2C2C),
            activity_bar_foreground: Color::hex(0xFFFFFF),
            activity_bar_badge: Color::hex(0x007ACC),
            activity_bar_badge_foreground: Color::hex(0xFFFFFF),

            // Side bar
            sidebar_background: Color::hex(0xF3F3F3),
            sidebar_foreground: Color::hex(0x333333),
            sidebar_section_header: Color::hex(0xE7E7E7),

            // Title bar
            title_bar_background: Color::hex(0xDDDDDD),
            title_bar_foreground: Color::hex(0x333333),

            // Tabs
            tab_active_background: Color::hex(0xFFFFFF),
            tab_active_foreground: Color::hex(0x333333),
            tab_inactive_background: Color::hex(0xECECEC),
            tab_inactive_foreground: Color::hex(0x8E8E8E),
            tab_border: Color::hex(0xF3F3F3),

            // Status bar
            status_bar_background: Color::hex(0x007ACC),
            status_bar_foreground: Color::hex(0xFFFFFF),
            status_bar_debugging: Color::hex(0xCC6633),
            status_bar_no_folder: Color::hex(0x68217A),

            // Panel
            panel_background: Color::hex(0xFFFFFF),
            panel_foreground: Color::hex(0x333333),
            panel_border: Color::hex(0xE5E5E5),

            // Terminal
            terminal_background: Color::hex(0xFFFFFF),
            terminal_foreground: Color::hex(0x333333),
            terminal_cursor: Color::hex(0x000000),
            terminal_selection: Color::rgba(0, 0, 0, 50),

            // ANSI (light versions)
            terminal_black: Color::hex(0x000000),
            terminal_red: Color::hex(0xCD3131),
            terminal_green: Color::hex(0x00BC00),
            terminal_yellow: Color::hex(0x949800),
            terminal_blue: Color::hex(0x0451A5),
            terminal_magenta: Color::hex(0xBC05BC),
            terminal_cyan: Color::hex(0x0598BC),
            terminal_white: Color::hex(0x555555),
            terminal_bright_black: Color::hex(0x666666),
            terminal_bright_red: Color::hex(0xCD3131),
            terminal_bright_green: Color::hex(0x14CE14),
            terminal_bright_yellow: Color::hex(0xB5BA00),
            terminal_bright_blue: Color::hex(0x0451A5),
            terminal_bright_magenta: Color::hex(0xBC05BC),
            terminal_bright_cyan: Color::hex(0x0598BC),
            terminal_bright_white: Color::hex(0xA5A5A5),

            // Button
            button_background: Color::hex(0x007ACC),
            button_foreground: Color::hex(0xFFFFFF),
            button_hover: Color::hex(0x0062A3),
            button_secondary_background: Color::hex(0xE5E5E5),
            button_secondary_foreground: Color::hex(0x333333),

            // Input
            input_background: Color::hex(0xFFFFFF),
            input_foreground: Color::hex(0x333333),
            input_border: Color::hex(0xCECECE),
            input_placeholder: Color::hex(0x767676),

            // List
            list_active_background: Color::hex(0x0060C0),
            list_active_foreground: Color::hex(0xFFFFFF),
            list_hover_background: Color::hex(0xE8E8E8),
            list_focus_background: Color::hex(0xD6EBFF),

            // Accent
            accent: Color::hex(0x007ACC),
            accent_foreground: Color::hex(0xFFFFFF),
            link: Color::hex(0x006AB1),
            error: Color::hex(0xE51400),
            warning: Color::hex(0xBF8803),
            info: Color::hex(0x1A85FF),
            success: Color::hex(0x388A34),

            // Git
            git_added: Color::hex(0x587C0C),
            git_modified: Color::hex(0x895503),
            git_deleted: Color::hex(0xAD0707),
            git_untracked: Color::hex(0x007100),
            git_ignored: Color::hex(0x8C8C8C),
            git_conflict: Color::hex(0xE4676B),
        }
    }

    /// Create default theme for kind
    pub fn default_for(kind: ThemeKind) -> Self {
        match kind {
            ThemeKind::Dark | ThemeKind::HighContrastDark => Self::dark(),
            ThemeKind::Light | ThemeKind::HighContrastLight => Self::light(),
        }
    }
}

impl Default for UiTheme {
    fn default() -> Self {
        Self::dark()
    }
}
