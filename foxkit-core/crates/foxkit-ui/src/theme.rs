//! Theme System
//!
//! Unified theming for Foxkit IDE with colors, fonts, and styling.

use crate::components::activity_bar::Color;

/// Main theme structure
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub kind: ThemeKind,
    pub colors: ThemeColors,
    pub fonts: ThemeFonts,
    pub spacing: ThemeSpacing,
    pub borders: ThemeBorders,
}

impl Theme {
    /// Create the default dark theme (VS Code Dark+)
    pub fn dark() -> Self {
        Self {
            name: "Foxkit Dark".into(),
            kind: ThemeKind::Dark,
            colors: ThemeColors::dark(),
            fonts: ThemeFonts::default(),
            spacing: ThemeSpacing::default(),
            borders: ThemeBorders::default(),
        }
    }

    /// Create a light theme
    pub fn light() -> Self {
        Self {
            name: "Foxkit Light".into(),
            kind: ThemeKind::Light,
            colors: ThemeColors::light(),
            fonts: ThemeFonts::default(),
            spacing: ThemeSpacing::default(),
            borders: ThemeBorders::default(),
        }
    }

    /// Create a high contrast theme
    pub fn high_contrast() -> Self {
        Self {
            name: "Foxkit High Contrast".into(),
            kind: ThemeKind::HighContrast,
            colors: ThemeColors::high_contrast(),
            fonts: ThemeFonts::default(),
            spacing: ThemeSpacing::default(),
            borders: ThemeBorders::default(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeKind {
    Light,
    Dark,
    HighContrast,
}

/// All theme colors
#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    // Base colors
    pub foreground: Color,
    pub background: Color,
    pub border: Color,
    pub focus_border: Color,
    pub selection: Color,
    pub inactive_selection: Color,

    // Activity bar
    pub activity_bar_bg: Color,
    pub activity_bar_fg: Color,
    pub activity_bar_inactive_fg: Color,
    pub activity_bar_badge_bg: Color,
    pub activity_bar_badge_fg: Color,
    pub activity_bar_active_border: Color,

    // Sidebar
    pub sidebar_bg: Color,
    pub sidebar_fg: Color,
    pub sidebar_header_bg: Color,
    pub sidebar_header_fg: Color,
    pub sidebar_section_header_bg: Color,
    pub sidebar_section_header_fg: Color,
    pub sidebar_item_hover_bg: Color,
    pub sidebar_item_active_bg: Color,
    pub sidebar_border: Color,

    // Editor
    pub editor_bg: Color,
    pub editor_fg: Color,
    pub editor_line_number_fg: Color,
    pub editor_line_number_active_fg: Color,
    pub editor_cursor: Color,
    pub editor_selection_bg: Color,
    pub editor_current_line_bg: Color,
    pub editor_whitespace_fg: Color,
    pub editor_indent_guide: Color,
    pub editor_indent_guide_active: Color,

    // Editor groups
    pub editor_group_header_bg: Color,
    pub editor_group_border: Color,
    pub editor_gutter_bg: Color,

    // Tabs
    pub tab_bar_bg: Color,
    pub tab_active_bg: Color,
    pub tab_active_fg: Color,
    pub tab_active_border: Color,
    pub tab_inactive_bg: Color,
    pub tab_inactive_fg: Color,
    pub tab_hover_bg: Color,
    pub tab_modified_dot: Color,
    pub tab_border: Color,

    // Terminal
    pub terminal_bg: Color,
    pub terminal_fg: Color,
    pub terminal_cursor: Color,
    pub terminal_selection_bg: Color,
    pub terminal_ansi_black: Color,
    pub terminal_ansi_red: Color,
    pub terminal_ansi_green: Color,
    pub terminal_ansi_yellow: Color,
    pub terminal_ansi_blue: Color,
    pub terminal_ansi_magenta: Color,
    pub terminal_ansi_cyan: Color,
    pub terminal_ansi_white: Color,
    pub terminal_ansi_bright_black: Color,
    pub terminal_ansi_bright_red: Color,
    pub terminal_ansi_bright_green: Color,
    pub terminal_ansi_bright_yellow: Color,
    pub terminal_ansi_bright_blue: Color,
    pub terminal_ansi_bright_magenta: Color,
    pub terminal_ansi_bright_cyan: Color,
    pub terminal_ansi_bright_white: Color,

    // Status bar
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
    pub status_bar_debugging_bg: Color,
    pub status_bar_no_folder_bg: Color,
    pub status_bar_remote_bg: Color,
    pub status_bar_item_hover_bg: Color,

    // Panel (bottom)
    pub panel_bg: Color,
    pub panel_fg: Color,
    pub panel_border: Color,
    pub panel_header_bg: Color,
    pub panel_tab_bar_bg: Color,
    pub panel_tab_active_bg: Color,
    pub panel_tab_active_fg: Color,
    pub panel_tab_inactive_bg: Color,
    pub panel_tab_inactive_fg: Color,

    // Input controls
    pub input_bg: Color,
    pub input_fg: Color,
    pub input_border: Color,
    pub input_focus_border: Color,
    pub input_placeholder_fg: Color,

    // Buttons
    pub button_bg: Color,
    pub button_fg: Color,
    pub button_hover_bg: Color,
    pub button_secondary_bg: Color,
    pub button_secondary_fg: Color,

    // Lists and trees
    pub list_hover_bg: Color,
    pub list_active_selection_bg: Color,
    pub list_active_selection_fg: Color,
    pub list_inactive_selection_bg: Color,
    pub list_focus_bg: Color,
    pub list_active_bg: Color,
    pub list_active_fg: Color,

    // Scrollbar
    pub scrollbar_slider_bg: Color,
    pub scrollbar_slider_hover_bg: Color,
    pub scrollbar_slider_active_bg: Color,

    // Notifications
    pub notification_bg: Color,
    pub notification_fg: Color,
    pub notification_border: Color,

    // Problems/Diagnostics
    pub error_fg: Color,
    pub warning_fg: Color,
    pub info_fg: Color,
    pub hint_fg: Color,

    // AI/Chat
    pub ai_accent: Color,
    pub ai_bubble_user_bg: Color,
    pub ai_bubble_assistant_bg: Color,
    pub chat_user_bg: Color,
    pub chat_user_fg: Color,
    pub chat_assistant_bg: Color,
    pub chat_assistant_fg: Color,
    pub chat_system_bg: Color,
    pub chat_system_fg: Color,

    // Git colors
    pub git_added: Color,
    pub git_modified: Color,
    pub git_deleted: Color,
    pub git_untracked: Color,
    pub git_ignored: Color,
    pub git_conflict: Color,

    // Diff colors
    pub diff_inserted_bg: Color,
    pub diff_removed_bg: Color,
    pub diff_changed_bg: Color,

    // Minimap
    pub minimap_bg: Color,
    pub minimap_selection_highlight: Color,
    pub minimap_find_match: Color,

    // Breadcrumb
    pub breadcrumb_bg: Color,
    pub breadcrumb_fg: Color,
    pub breadcrumb_focus_fg: Color,
}

impl ThemeColors {
    /// Dark theme colors (VS Code Dark+ inspired)
    pub fn dark() -> Self {
        Self {
            // Base
            foreground: Color::hex(0xCCCCCC),
            background: Color::hex(0x1E1E1E),
            border: Color::hex(0x3C3C3C),
            focus_border: Color::hex(0x007FD4),
            selection: Color::hex(0x264F78),
            inactive_selection: Color::hex(0x3A3D41),

            // Activity bar
            activity_bar_bg: Color::hex(0x333333),
            activity_bar_fg: Color::hex(0xFFFFFF),
            activity_bar_inactive_fg: Color::hex(0x858585),
            activity_bar_badge_bg: Color::hex(0x007ACC),
            activity_bar_badge_fg: Color::hex(0xFFFFFF),
            activity_bar_active_border: Color::hex(0xFFFFFF),

            // Sidebar
            sidebar_bg: Color::hex(0x252526),
            sidebar_fg: Color::hex(0xCCCCCC),
            sidebar_header_bg: Color::hex(0x252526),
            sidebar_header_fg: Color::hex(0xBBBBBB),
            sidebar_section_header_bg: Color::hex(0x37373D),
            sidebar_section_header_fg: Color::hex(0xBBBBBB),
            sidebar_item_hover_bg: Color::hex(0x2A2D2E),
            sidebar_item_active_bg: Color::hex(0x37373D),
            sidebar_border: Color::hex(0x3C3C3C),

            // Editor
            editor_bg: Color::hex(0x1E1E1E),
            editor_fg: Color::hex(0xD4D4D4),
            editor_line_number_fg: Color::hex(0x858585),
            editor_line_number_active_fg: Color::hex(0xC6C6C6),
            editor_cursor: Color::hex(0xAEAFAD),
            editor_selection_bg: Color::hex(0x264F78),
            editor_current_line_bg: Color::hex(0x282828),
            editor_whitespace_fg: Color::hex(0x3B3B3B),
            editor_indent_guide: Color::hex(0x404040),
            editor_indent_guide_active: Color::hex(0x707070),

            // Editor groups
            editor_group_header_bg: Color::hex(0x252526),
            editor_group_border: Color::hex(0x444444),
            editor_gutter_bg: Color::hex(0x1E1E1E),

            // Tabs
            tab_bar_bg: Color::hex(0x252526),
            tab_active_bg: Color::hex(0x1E1E1E),
            tab_active_fg: Color::hex(0xFFFFFF),
            tab_active_border: Color::hex(0x007ACC),
            tab_inactive_bg: Color::hex(0x2D2D2D),
            tab_inactive_fg: Color::hex(0x969696),
            tab_hover_bg: Color::hex(0x2D2D2D),
            tab_modified_dot: Color::hex(0xE2C08D),
            tab_border: Color::hex(0x252526),

            // Terminal
            terminal_bg: Color::hex(0x1E1E1E),
            terminal_fg: Color::hex(0xCCCCCC),
            terminal_cursor: Color::hex(0xFFFFFF),
            terminal_selection_bg: Color::hex(0x264F78),
            terminal_ansi_black: Color::hex(0x000000),
            terminal_ansi_red: Color::hex(0xCD3131),
            terminal_ansi_green: Color::hex(0x0DBC79),
            terminal_ansi_yellow: Color::hex(0xE5E510),
            terminal_ansi_blue: Color::hex(0x2472C8),
            terminal_ansi_magenta: Color::hex(0xBC3FBC),
            terminal_ansi_cyan: Color::hex(0x11A8CD),
            terminal_ansi_white: Color::hex(0xE5E5E5),
            terminal_ansi_bright_black: Color::hex(0x666666),
            terminal_ansi_bright_red: Color::hex(0xF14C4C),
            terminal_ansi_bright_green: Color::hex(0x23D18B),
            terminal_ansi_bright_yellow: Color::hex(0xF5F543),
            terminal_ansi_bright_blue: Color::hex(0x3B8EEA),
            terminal_ansi_bright_magenta: Color::hex(0xD670D6),
            terminal_ansi_bright_cyan: Color::hex(0x29B8DB),
            terminal_ansi_bright_white: Color::hex(0xFFFFFF),

            // Status bar
            status_bar_bg: Color::hex(0x007ACC),
            status_bar_fg: Color::hex(0xFFFFFF),
            status_bar_debugging_bg: Color::hex(0xCC6633),
            status_bar_no_folder_bg: Color::hex(0x68217A),
            status_bar_remote_bg: Color::hex(0x16825D),
            status_bar_item_hover_bg: Color::rgba(255, 255, 255, 0.12),

            // Panel
            panel_bg: Color::hex(0x1E1E1E),
            panel_fg: Color::hex(0xCCCCCC),
            panel_border: Color::hex(0x3C3C3C),
            panel_header_bg: Color::hex(0x252526),
            panel_tab_bar_bg: Color::hex(0x252526),
            panel_tab_active_bg: Color::hex(0x1E1E1E),
            panel_tab_active_fg: Color::hex(0xFFFFFF),
            panel_tab_inactive_bg: Color::hex(0x252526),
            panel_tab_inactive_fg: Color::hex(0x969696),

            // Input
            input_bg: Color::hex(0x3C3C3C),
            input_fg: Color::hex(0xCCCCCC),
            input_border: Color::hex(0x3C3C3C),
            input_focus_border: Color::hex(0x007FD4),
            input_placeholder_fg: Color::hex(0x858585),

            // Buttons
            button_bg: Color::hex(0x0E639C),
            button_fg: Color::hex(0xFFFFFF),
            button_hover_bg: Color::hex(0x1177BB),
            button_secondary_bg: Color::hex(0x3A3D41),
            button_secondary_fg: Color::hex(0xCCCCCC),

            // Lists
            list_hover_bg: Color::hex(0x2A2D2E),
            list_active_selection_bg: Color::hex(0x094771),
            list_active_selection_fg: Color::hex(0xFFFFFF),
            list_inactive_selection_bg: Color::hex(0x37373D),
            list_focus_bg: Color::hex(0x062F4A),
            list_active_bg: Color::hex(0x094771),
            list_active_fg: Color::hex(0xFFFFFF),

            // Scrollbar
            scrollbar_slider_bg: Color::rgba(121, 121, 121, 0.4),
            scrollbar_slider_hover_bg: Color::rgba(100, 100, 100, 0.7),
            scrollbar_slider_active_bg: Color::rgba(191, 191, 191, 0.4),

            // Notifications
            notification_bg: Color::hex(0x252526),
            notification_fg: Color::hex(0xCCCCCC),
            notification_border: Color::hex(0x3C3C3C),

            // Diagnostics
            error_fg: Color::hex(0xF48771),
            warning_fg: Color::hex(0xCCA700),
            info_fg: Color::hex(0x75BEFF),
            hint_fg: Color::hex(0xEEEEEE),

            // AI
            ai_accent: Color::hex(0x6B57FF),
            ai_bubble_user_bg: Color::hex(0x094771),
            ai_bubble_assistant_bg: Color::hex(0x37373D),
            chat_user_bg: Color::hex(0x094771),
            chat_user_fg: Color::hex(0xFFFFFF),
            chat_assistant_bg: Color::hex(0x37373D),
            chat_assistant_fg: Color::hex(0xCCCCCC),
            chat_system_bg: Color::hex(0x3C3C3C),
            chat_system_fg: Color::hex(0x808080),

            // Git
            git_added: Color::hex(0x587C0C),
            git_modified: Color::hex(0x895503),
            git_deleted: Color::hex(0x94151B),
            git_untracked: Color::hex(0x388A34),
            git_ignored: Color::hex(0x8C8C8C),
            git_conflict: Color::hex(0xFF0000),

            // Diff
            diff_inserted_bg: Color::rgba(155, 185, 85, 0.2),
            diff_removed_bg: Color::rgba(255, 0, 0, 0.2),
            diff_changed_bg: Color::rgba(0, 100, 200, 0.2),

            // Minimap
            minimap_bg: Color::hex(0x1E1E1E),
            minimap_selection_highlight: Color::rgba(255, 255, 255, 0.3),
            minimap_find_match: Color::rgba(234, 92, 0, 0.5),

            // Breadcrumb
            breadcrumb_bg: Color::hex(0x1E1E1E),
            breadcrumb_fg: Color::hex(0xCCCCCC),
            breadcrumb_focus_fg: Color::hex(0xFFFFFF),
        }
    }

    /// Light theme colors
    pub fn light() -> Self {
        Self {
            // Base
            foreground: Color::hex(0x333333),
            background: Color::hex(0xFFFFFF),
            border: Color::hex(0xE5E5E5),
            focus_border: Color::hex(0x007FD4),
            selection: Color::hex(0xADD6FF),
            inactive_selection: Color::hex(0xE5EBF1),

            // Activity bar
            activity_bar_bg: Color::hex(0x2C2C2C),
            activity_bar_fg: Color::hex(0xFFFFFF),
            activity_bar_inactive_fg: Color::hex(0x858585),
            activity_bar_badge_bg: Color::hex(0x007ACC),
            activity_bar_badge_fg: Color::hex(0xFFFFFF),
            activity_bar_active_border: Color::hex(0xFFFFFF),

            // Sidebar
            sidebar_bg: Color::hex(0xF3F3F3),
            sidebar_fg: Color::hex(0x333333),
            sidebar_header_bg: Color::hex(0xF3F3F3),
            sidebar_header_fg: Color::hex(0x333333),
            sidebar_section_header_bg: Color::hex(0xE7E7E7),
            sidebar_section_header_fg: Color::hex(0x333333),
            sidebar_item_hover_bg: Color::hex(0xE8E8E8),
            sidebar_item_active_bg: Color::hex(0xD6EBFF),
            sidebar_border: Color::hex(0xE5E5E5),

            // Editor
            editor_bg: Color::hex(0xFFFFFF),
            editor_fg: Color::hex(0x000000),
            editor_line_number_fg: Color::hex(0x237893),
            editor_line_number_active_fg: Color::hex(0x0B216F),
            editor_cursor: Color::hex(0x000000),
            editor_selection_bg: Color::hex(0xADD6FF),
            editor_current_line_bg: Color::hex(0xFFFF00),
            editor_whitespace_fg: Color::hex(0xD3D3D3),
            editor_indent_guide: Color::hex(0xD3D3D3),
            editor_indent_guide_active: Color::hex(0x939393),

            // Editor groups
            editor_group_header_bg: Color::hex(0xF3F3F3),
            editor_group_border: Color::hex(0xE7E7E7),
            editor_gutter_bg: Color::hex(0xFFFFFF),

            // Tabs
            tab_bar_bg: Color::hex(0xF3F3F3),
            tab_active_bg: Color::hex(0xFFFFFF),
            tab_active_fg: Color::hex(0x333333),
            tab_active_border: Color::hex(0x007ACC),
            tab_inactive_bg: Color::hex(0xECECEC),
            tab_inactive_fg: Color::hex(0x5A5A5A),
            tab_hover_bg: Color::hex(0xF3F3F3),
            tab_modified_dot: Color::hex(0x000000),
            tab_border: Color::hex(0xE5E5E5),

            // Terminal
            terminal_bg: Color::hex(0xFFFFFF),
            terminal_fg: Color::hex(0x333333),
            terminal_cursor: Color::hex(0x000000),
            terminal_selection_bg: Color::hex(0xADD6FF),
            terminal_ansi_black: Color::hex(0x000000),
            terminal_ansi_red: Color::hex(0xCD3131),
            terminal_ansi_green: Color::hex(0x00BC00),
            terminal_ansi_yellow: Color::hex(0x949800),
            terminal_ansi_blue: Color::hex(0x0451A5),
            terminal_ansi_magenta: Color::hex(0xBC05BC),
            terminal_ansi_cyan: Color::hex(0x0598BC),
            terminal_ansi_white: Color::hex(0x555555),
            terminal_ansi_bright_black: Color::hex(0x666666),
            terminal_ansi_bright_red: Color::hex(0xCD3131),
            terminal_ansi_bright_green: Color::hex(0x14CE14),
            terminal_ansi_bright_yellow: Color::hex(0xB5BA00),
            terminal_ansi_bright_blue: Color::hex(0x0451A5),
            terminal_ansi_bright_magenta: Color::hex(0xBC05BC),
            terminal_ansi_bright_cyan: Color::hex(0x0598BC),
            terminal_ansi_bright_white: Color::hex(0xA5A5A5),

            // Status bar
            status_bar_bg: Color::hex(0x007ACC),
            status_bar_fg: Color::hex(0xFFFFFF),
            status_bar_debugging_bg: Color::hex(0xCC6633),
            status_bar_no_folder_bg: Color::hex(0x68217A),
            status_bar_remote_bg: Color::hex(0x16825D),
            status_bar_item_hover_bg: Color::rgba(255, 255, 255, 0.12),

            // Panel
            panel_bg: Color::hex(0xFFFFFF),
            panel_tab_bar_bg: Color::hex(0xF3F3F3),
            panel_tab_active_bg: Color::hex(0xFFFFFF),
            panel_tab_active_fg: Color::hex(0x333333),
            panel_tab_inactive_bg: Color::hex(0xF3F3F3),
            panel_tab_inactive_fg: Color::hex(0x5A5A5A),
            panel_fg: Color::hex(0x333333),
            panel_border: Color::hex(0xE5E5E5),
            panel_header_bg: Color::hex(0xF3F3F3),

            // Input
            input_bg: Color::hex(0xFFFFFF),
            input_fg: Color::hex(0x333333),
            input_border: Color::hex(0xCECECE),
            input_focus_border: Color::hex(0x007FD4),
            input_placeholder_fg: Color::hex(0x767676),

            // Buttons
            button_bg: Color::hex(0x007ACC),
            button_fg: Color::hex(0xFFFFFF),
            button_hover_bg: Color::hex(0x0066B8),
            button_secondary_bg: Color::hex(0xF3F3F3),
            button_secondary_fg: Color::hex(0x333333),

            // Lists
            list_hover_bg: Color::hex(0xE8E8E8),
            list_active_selection_bg: Color::hex(0x0060C0),
            list_active_selection_fg: Color::hex(0xFFFFFF),
            list_inactive_selection_bg: Color::hex(0xE4E6F1),
            list_focus_bg: Color::hex(0xC4D7E9),
            list_active_bg: Color::hex(0x0060C0),
            list_active_fg: Color::hex(0xFFFFFF),

            // Scrollbar
            scrollbar_slider_bg: Color::rgba(100, 100, 100, 0.4),
            scrollbar_slider_hover_bg: Color::rgba(100, 100, 100, 0.7),
            scrollbar_slider_active_bg: Color::rgba(0, 0, 0, 0.6),

            // Notifications
            notification_bg: Color::hex(0xFFFFFF),
            notification_fg: Color::hex(0x333333),
            notification_border: Color::hex(0xE5E5E5),

            // Diagnostics
            error_fg: Color::hex(0xE51400),
            warning_fg: Color::hex(0xBF8803),
            info_fg: Color::hex(0x1A85FF),
            hint_fg: Color::hex(0x6C6C6C),

            // AI
            ai_accent: Color::hex(0x6B57FF),
            ai_bubble_user_bg: Color::hex(0xD6EBFF),
            ai_bubble_assistant_bg: Color::hex(0xF3F3F3),
            chat_user_bg: Color::hex(0xD6EBFF),
            chat_user_fg: Color::hex(0x000000),
            chat_assistant_bg: Color::hex(0xF3F3F3),
            chat_assistant_fg: Color::hex(0x333333),
            chat_system_bg: Color::hex(0xE8E8E8),
            chat_system_fg: Color::hex(0x666666),

            // Git
            git_added: Color::hex(0x587C0C),
            git_modified: Color::hex(0x895503),
            git_deleted: Color::hex(0x94151B),
            git_untracked: Color::hex(0x388A34),
            git_ignored: Color::hex(0x8C8C8C),
            git_conflict: Color::hex(0xFF0000),

            // Diff
            diff_inserted_bg: Color::rgba(155, 185, 85, 0.2),
            diff_removed_bg: Color::rgba(255, 0, 0, 0.2),
            diff_changed_bg: Color::rgba(0, 100, 200, 0.2),

            // Minimap
            minimap_bg: Color::hex(0xFFFFFF),
            minimap_selection_highlight: Color::rgba(0, 0, 0, 0.3),
            minimap_find_match: Color::rgba(234, 92, 0, 0.5),

            // Breadcrumb
            breadcrumb_bg: Color::hex(0xFFFFFF),
            breadcrumb_fg: Color::hex(0x333333),
            breadcrumb_focus_fg: Color::hex(0x000000),
        }
    }

    /// High contrast theme colors
    pub fn high_contrast() -> Self {
        Self {
            // Base
            foreground: Color::hex(0xFFFFFF),
            background: Color::hex(0x000000),
            border: Color::hex(0x6FC3DF),
            focus_border: Color::hex(0xF38518),
            selection: Color::hex(0xFFFFFF),
            inactive_selection: Color::hex(0x3F3F3F),

            // Activity bar
            activity_bar_bg: Color::hex(0x000000),
            activity_bar_fg: Color::hex(0xFFFFFF),
            activity_bar_inactive_fg: Color::hex(0xFFFFFF),
            activity_bar_badge_bg: Color::hex(0x6FC3DF),
            activity_bar_badge_fg: Color::hex(0x000000),
            activity_bar_active_border: Color::hex(0xFFFFFF),

            // Sidebar
            sidebar_bg: Color::hex(0x000000),
            sidebar_fg: Color::hex(0xFFFFFF),
            sidebar_header_bg: Color::hex(0x000000),
            sidebar_header_fg: Color::hex(0xFFFFFF),
            sidebar_section_header_bg: Color::hex(0x000000),
            sidebar_section_header_fg: Color::hex(0xFFFFFF),
            sidebar_item_hover_bg: Color::hex(0x000000),
            sidebar_item_active_bg: Color::hex(0x000000),
            sidebar_border: Color::hex(0x6FC3DF),

            // Editor
            editor_bg: Color::hex(0x000000),
            editor_fg: Color::hex(0xFFFFFF),
            editor_line_number_fg: Color::hex(0x00FF00),
            editor_line_number_active_fg: Color::hex(0x00FF00),
            editor_cursor: Color::hex(0xFFFFFF),
            editor_selection_bg: Color::hex(0xFFFFFF),
            editor_current_line_bg: Color::hex(0x000000),
            editor_whitespace_fg: Color::hex(0x7F7F7F),
            editor_indent_guide: Color::hex(0x6FC3DF),
            editor_indent_guide_active: Color::hex(0x6FC3DF),

            // Editor groups
            editor_group_header_bg: Color::hex(0x000000),
            editor_group_border: Color::hex(0x6FC3DF),
            editor_gutter_bg: Color::hex(0x000000),

            // Tabs
            tab_bar_bg: Color::hex(0x000000),
            tab_active_bg: Color::hex(0x000000),
            tab_active_fg: Color::hex(0xFFFFFF),
            tab_active_border: Color::hex(0x6FC3DF),
            tab_inactive_bg: Color::hex(0x000000),
            tab_inactive_fg: Color::hex(0xFFFFFF),
            tab_hover_bg: Color::hex(0x000000),
            tab_modified_dot: Color::hex(0xFFFFFF),
            tab_border: Color::hex(0x6FC3DF),

            // Terminal
            terminal_bg: Color::hex(0x000000),
            terminal_fg: Color::hex(0xFFFFFF),
            terminal_cursor: Color::hex(0xFFFFFF),
            terminal_selection_bg: Color::hex(0xFFFFFF),
            terminal_ansi_black: Color::hex(0x000000),
            terminal_ansi_red: Color::hex(0xFF0000),
            terminal_ansi_green: Color::hex(0x00FF00),
            terminal_ansi_yellow: Color::hex(0xFFFF00),
            terminal_ansi_blue: Color::hex(0x0000FF),
            terminal_ansi_magenta: Color::hex(0xFF00FF),
            terminal_ansi_cyan: Color::hex(0x00FFFF),
            terminal_ansi_white: Color::hex(0xFFFFFF),
            terminal_ansi_bright_black: Color::hex(0x808080),
            terminal_ansi_bright_red: Color::hex(0xFF0000),
            terminal_ansi_bright_green: Color::hex(0x00FF00),
            terminal_ansi_bright_yellow: Color::hex(0xFFFF00),
            terminal_ansi_bright_blue: Color::hex(0x0000FF),
            terminal_ansi_bright_magenta: Color::hex(0xFF00FF),
            terminal_ansi_bright_cyan: Color::hex(0x00FFFF),
            terminal_ansi_bright_white: Color::hex(0xFFFFFF),

            // Status bar
            status_bar_bg: Color::hex(0x000000),
            status_bar_fg: Color::hex(0xFFFFFF),
            status_bar_debugging_bg: Color::hex(0x6FC3DF),
            status_bar_no_folder_bg: Color::hex(0x6FC3DF),
            status_bar_remote_bg: Color::hex(0x6FC3DF),
            status_bar_item_hover_bg: Color::hex(0x000000),

            // Panel
            panel_bg: Color::hex(0x000000),
            panel_fg: Color::hex(0xFFFFFF),
            panel_border: Color::hex(0x6FC3DF),
            panel_header_bg: Color::hex(0x000000),
            panel_tab_bar_bg: Color::hex(0x000000),
            panel_tab_active_bg: Color::hex(0x000000),
            panel_tab_active_fg: Color::hex(0xFFFFFF),
            panel_tab_inactive_bg: Color::hex(0x000000),
            panel_tab_inactive_fg: Color::hex(0xFFFFFF),

            // Input
            input_bg: Color::hex(0x000000),
            input_fg: Color::hex(0xFFFFFF),
            input_border: Color::hex(0x6FC3DF),
            input_focus_border: Color::hex(0xF38518),
            input_placeholder_fg: Color::hex(0xFFFFFF),

            // Buttons
            button_bg: Color::hex(0x000000),
            button_fg: Color::hex(0xFFFFFF),
            button_hover_bg: Color::hex(0x000000),
            button_secondary_bg: Color::hex(0x000000),
            button_secondary_fg: Color::hex(0xFFFFFF),

            // Lists
            list_hover_bg: Color::hex(0x000000),
            list_active_selection_bg: Color::hex(0x000000),
            list_active_selection_fg: Color::hex(0xFFFFFF),
            list_inactive_selection_bg: Color::hex(0x000000),
            list_focus_bg: Color::hex(0x000000),
            list_active_bg: Color::hex(0x000000),
            list_active_fg: Color::hex(0xFFFFFF),

            // Scrollbar
            scrollbar_slider_bg: Color::hex(0x6FC3DF),
            scrollbar_slider_hover_bg: Color::hex(0x6FC3DF),
            scrollbar_slider_active_bg: Color::hex(0x6FC3DF),

            // Notifications
            notification_bg: Color::hex(0x000000),
            notification_fg: Color::hex(0xFFFFFF),
            notification_border: Color::hex(0x6FC3DF),

            // Diagnostics
            error_fg: Color::hex(0xFF0000),
            warning_fg: Color::hex(0xFFFF00),
            info_fg: Color::hex(0x00FFFF),
            hint_fg: Color::hex(0xFFFFFF),

            // AI
            ai_accent: Color::hex(0xF38518),
            ai_bubble_user_bg: Color::hex(0x000000),
            ai_bubble_assistant_bg: Color::hex(0x000000),
            chat_user_bg: Color::hex(0x000000),
            chat_user_fg: Color::hex(0xFFFFFF),
            chat_assistant_bg: Color::hex(0x000000),
            chat_assistant_fg: Color::hex(0xFFFFFF),
            chat_system_bg: Color::hex(0x000000),
            chat_system_fg: Color::hex(0xFFFFFF),

            // Git
            git_added: Color::hex(0x00FF00),
            git_modified: Color::hex(0xFFFF00),
            git_deleted: Color::hex(0xFF0000),
            git_untracked: Color::hex(0x00FF00),
            git_ignored: Color::hex(0x808080),
            git_conflict: Color::hex(0xFF0000),

            // Diff
            diff_inserted_bg: Color::rgba(0, 255, 0, 0.3),
            diff_removed_bg: Color::rgba(255, 0, 0, 0.3),
            diff_changed_bg: Color::rgba(0, 0, 255, 0.3),

            // Minimap
            minimap_bg: Color::hex(0x000000),
            minimap_selection_highlight: Color::hex(0xFFFFFF),
            minimap_find_match: Color::hex(0xF38518),

            // Breadcrumb
            breadcrumb_bg: Color::hex(0x000000),
            breadcrumb_fg: Color::hex(0xFFFFFF),
            breadcrumb_focus_fg: Color::hex(0xFFFFFF),
        }
    }
}

/// Theme fonts configuration
#[derive(Debug, Clone)]
pub struct ThemeFonts {
    pub editor_family: String,
    pub editor_size: f32,
    pub editor_line_height: f32,
    pub ui_family: String,
    pub ui_size: f32,
    pub terminal_family: String,
    pub terminal_size: f32,
}

impl Default for ThemeFonts {
    fn default() -> Self {
        Self {
            editor_family: "Fira Code, Consolas, monospace".into(),
            editor_size: 14.0,
            editor_line_height: 1.5,
            ui_family: "Segoe UI, system-ui, sans-serif".into(),
            ui_size: 13.0,
            terminal_family: "Fira Code, Consolas, monospace".into(),
            terminal_size: 13.0,
        }
    }
}

/// Theme spacing configuration
#[derive(Debug, Clone, Copy)]
pub struct ThemeSpacing {
    pub activity_bar_width: f32,
    pub sidebar_width: f32,
    pub sidebar_min_width: f32,
    pub sidebar_max_width: f32,
    pub status_bar_height: f32,
    pub tab_height: f32,
    pub panel_header_height: f32,
    pub panel_min_height: f32,
    pub scrollbar_width: f32,
    pub minimap_width: f32,
}

impl Default for ThemeSpacing {
    fn default() -> Self {
        Self {
            activity_bar_width: 48.0,
            sidebar_width: 240.0,
            sidebar_min_width: 170.0,
            sidebar_max_width: 600.0,
            status_bar_height: 22.0,
            tab_height: 35.0,
            panel_header_height: 35.0,
            panel_min_height: 100.0,
            scrollbar_width: 10.0,
            minimap_width: 120.0,
        }
    }
}

/// Theme border configuration
#[derive(Debug, Clone, Copy)]
pub struct ThemeBorders {
    pub radius_small: f32,
    pub radius_medium: f32,
    pub radius_large: f32,
    pub width: f32,
}

impl Default for ThemeBorders {
    fn default() -> Self {
        Self {
            radius_small: 2.0,
            radius_medium: 4.0,
            radius_large: 8.0,
            width: 1.0,
        }
    }
}
