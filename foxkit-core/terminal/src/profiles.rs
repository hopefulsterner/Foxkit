//! Terminal profile configuration management.
//!
//! Provides a comprehensive system for managing terminal profiles including:
//! - Default shell configuration
//! - Environment variables
//! - Font and color settings
//! - Keyboard shortcuts
//! - Multiple named profiles

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A complete terminal profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalProfile {
    /// Unique identifier for this profile.
    pub id: String,
    /// Display name for the profile.
    pub name: String,
    /// Shell configuration.
    pub shell: ShellConfig,
    /// Environment variables to set.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Working directory (None = inherit).
    pub cwd: Option<PathBuf>,
    /// Terminal dimensions.
    pub size: TerminalSize,
    /// Font configuration.
    pub font: FontConfig,
    /// Color scheme.
    pub colors: ColorScheme,
    /// Cursor configuration.
    pub cursor: CursorConfig,
    /// Scrollback configuration.
    pub scrollback: ScrollbackConfig,
    /// Bell configuration.
    pub bell: BellConfig,
    /// Whether this is the default profile.
    #[serde(default)]
    pub is_default: bool,
    /// Icon for the profile (optional).
    pub icon: Option<String>,
    /// Tags for organization.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl TerminalProfile {
    /// Create a new profile with the given name and default shell.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            shell: ShellConfig::detect(),
            env: HashMap::new(),
            cwd: None,
            size: TerminalSize::default(),
            font: FontConfig::default(),
            colors: ColorScheme::default(),
            cursor: CursorConfig::default(),
            scrollback: ScrollbackConfig::default(),
            bell: BellConfig::default(),
            is_default: false,
            icon: None,
            tags: Vec::new(),
        }
    }

    /// Create a profile with a specific shell.
    pub fn with_shell(mut self, shell: ShellConfig) -> Self {
        self.shell = shell;
        self
    }

    /// Add an environment variable.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set the working directory.
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Set the color scheme.
    pub fn with_colors(mut self, colors: ColorScheme) -> Self {
        self.colors = colors;
        self
    }

    /// Mark as the default profile.
    pub fn as_default(mut self) -> Self {
        self.is_default = true;
        self
    }
}

impl Default for TerminalProfile {
    fn default() -> Self {
        Self::new("default", "Default")
    }
}

/// Shell configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    /// Path to the shell executable.
    pub path: PathBuf,
    /// Arguments to pass to the shell.
    #[serde(default)]
    pub args: Vec<String>,
    /// Whether to run as a login shell.
    #[serde(default)]
    pub login: bool,
}

impl ShellConfig {
    /// Create a new shell config.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            args: Vec::new(),
            login: false,
        }
    }

    /// Create a bash shell config.
    pub fn bash() -> Self {
        Self::new("/bin/bash")
    }

    /// Create a zsh shell config.
    pub fn zsh() -> Self {
        Self::new("/bin/zsh")
    }

    /// Create a fish shell config.
    pub fn fish() -> Self {
        Self::new("/usr/bin/fish")
    }

    /// Create a PowerShell config.
    pub fn powershell() -> Self {
        Self::new("pwsh")
    }

    /// Create a cmd.exe config.
    pub fn cmd() -> Self {
        Self::new("cmd.exe")
    }

    /// Detect the default shell for the current platform.
    pub fn detect() -> Self {
        #[cfg(unix)]
        {
            if let Ok(shell) = std::env::var("SHELL") {
                return Self::new(shell);
            }
            Self::bash()
        }

        #[cfg(windows)]
        {
            // Prefer PowerShell if available
            if std::process::Command::new("pwsh")
                .arg("--version")
                .output()
                .is_ok()
            {
                return Self::powershell();
            }
            Self::cmd()
        }

        #[cfg(not(any(unix, windows)))]
        {
            Self::new("/bin/sh")
        }
    }

    /// Add an argument.
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Set login shell mode.
    pub fn with_login(mut self, login: bool) -> Self {
        self.login = login;
        self
    }

    /// Get the full command with arguments.
    pub fn command_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        
        if self.login {
            // For login shells, typically -l flag
            let path_str = self.path.to_string_lossy();
            if path_str.contains("bash") || path_str.contains("zsh") {
                args.push("-l".to_string());
            } else if path_str.contains("fish") {
                args.push("--login".to_string());
            }
        }
        
        args.extend(self.args.clone());
        args
    }
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self::detect()
    }
}

/// Terminal dimensions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TerminalSize {
    /// Number of columns.
    pub cols: u16,
    /// Number of rows.
    pub rows: u16,
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self { cols: 80, rows: 24 }
    }
}

/// Font configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    /// Font family name.
    pub family: String,
    /// Font size in points.
    pub size: f32,
    /// Line height multiplier.
    pub line_height: f32,
    /// Letter spacing in pixels.
    pub letter_spacing: f32,
    /// Font weight (100-900).
    pub weight: u16,
    /// Whether to use ligatures.
    pub ligatures: bool,
    /// Fallback fonts.
    #[serde(default)]
    pub fallback: Vec<String>,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "monospace".to_string(),
            size: 14.0,
            line_height: 1.2,
            letter_spacing: 0.0,
            weight: 400,
            ligatures: true,
            fallback: vec![
                "Cascadia Code".to_string(),
                "Fira Code".to_string(),
                "JetBrains Mono".to_string(),
                "Consolas".to_string(),
                "Monaco".to_string(),
            ],
        }
    }
}

/// Color scheme definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    /// Scheme name.
    pub name: String,
    /// Foreground (text) color.
    pub foreground: RgbColor,
    /// Background color.
    pub background: RgbColor,
    /// Cursor color.
    pub cursor: RgbColor,
    /// Selection background color.
    pub selection_background: RgbColor,
    /// Selection foreground color (optional, use foreground if None).
    pub selection_foreground: Option<RgbColor>,
    /// ANSI colors (16 standard colors).
    pub ansi: AnsiColors,
}

impl Default for ColorScheme {
    fn default() -> Self {
        // Default dark theme
        Self {
            name: "Default Dark".to_string(),
            foreground: RgbColor::new(204, 204, 204),
            background: RgbColor::new(30, 30, 30),
            cursor: RgbColor::new(255, 255, 255),
            selection_background: RgbColor::new(68, 68, 68),
            selection_foreground: None,
            ansi: AnsiColors::default(),
        }
    }
}

impl ColorScheme {
    /// Create the "One Dark" color scheme.
    pub fn one_dark() -> Self {
        Self {
            name: "One Dark".to_string(),
            foreground: RgbColor::new(171, 178, 191),
            background: RgbColor::new(40, 44, 52),
            cursor: RgbColor::new(97, 175, 239),
            selection_background: RgbColor::new(62, 68, 81),
            selection_foreground: None,
            ansi: AnsiColors {
                black: RgbColor::new(40, 44, 52),
                red: RgbColor::new(224, 108, 117),
                green: RgbColor::new(152, 195, 121),
                yellow: RgbColor::new(229, 192, 123),
                blue: RgbColor::new(97, 175, 239),
                magenta: RgbColor::new(198, 120, 221),
                cyan: RgbColor::new(86, 182, 194),
                white: RgbColor::new(171, 178, 191),
                bright_black: RgbColor::new(92, 99, 112),
                bright_red: RgbColor::new(224, 108, 117),
                bright_green: RgbColor::new(152, 195, 121),
                bright_yellow: RgbColor::new(229, 192, 123),
                bright_blue: RgbColor::new(97, 175, 239),
                bright_magenta: RgbColor::new(198, 120, 221),
                bright_cyan: RgbColor::new(86, 182, 194),
                bright_white: RgbColor::new(255, 255, 255),
            },
        }
    }

    /// Create the "Dracula" color scheme.
    pub fn dracula() -> Self {
        Self {
            name: "Dracula".to_string(),
            foreground: RgbColor::new(248, 248, 242),
            background: RgbColor::new(40, 42, 54),
            cursor: RgbColor::new(248, 248, 242),
            selection_background: RgbColor::new(68, 71, 90),
            selection_foreground: None,
            ansi: AnsiColors {
                black: RgbColor::new(33, 34, 44),
                red: RgbColor::new(255, 85, 85),
                green: RgbColor::new(80, 250, 123),
                yellow: RgbColor::new(241, 250, 140),
                blue: RgbColor::new(189, 147, 249),
                magenta: RgbColor::new(255, 121, 198),
                cyan: RgbColor::new(139, 233, 253),
                white: RgbColor::new(248, 248, 242),
                bright_black: RgbColor::new(98, 114, 164),
                bright_red: RgbColor::new(255, 110, 110),
                bright_green: RgbColor::new(105, 255, 148),
                bright_yellow: RgbColor::new(255, 255, 165),
                bright_blue: RgbColor::new(214, 172, 255),
                bright_magenta: RgbColor::new(255, 146, 223),
                bright_cyan: RgbColor::new(164, 255, 255),
                bright_white: RgbColor::new(255, 255, 255),
            },
        }
    }

    /// Create the "Solarized Dark" color scheme.
    pub fn solarized_dark() -> Self {
        Self {
            name: "Solarized Dark".to_string(),
            foreground: RgbColor::new(131, 148, 150),
            background: RgbColor::new(0, 43, 54),
            cursor: RgbColor::new(131, 148, 150),
            selection_background: RgbColor::new(7, 54, 66),
            selection_foreground: None,
            ansi: AnsiColors {
                black: RgbColor::new(7, 54, 66),
                red: RgbColor::new(220, 50, 47),
                green: RgbColor::new(133, 153, 0),
                yellow: RgbColor::new(181, 137, 0),
                blue: RgbColor::new(38, 139, 210),
                magenta: RgbColor::new(211, 54, 130),
                cyan: RgbColor::new(42, 161, 152),
                white: RgbColor::new(238, 232, 213),
                bright_black: RgbColor::new(0, 43, 54),
                bright_red: RgbColor::new(203, 75, 22),
                bright_green: RgbColor::new(88, 110, 117),
                bright_yellow: RgbColor::new(101, 123, 131),
                bright_blue: RgbColor::new(131, 148, 150),
                bright_magenta: RgbColor::new(108, 113, 196),
                bright_cyan: RgbColor::new(147, 161, 161),
                bright_white: RgbColor::new(253, 246, 227),
            },
        }
    }

    /// Create a light color scheme.
    pub fn light() -> Self {
        Self {
            name: "Default Light".to_string(),
            foreground: RgbColor::new(30, 30, 30),
            background: RgbColor::new(255, 255, 255),
            cursor: RgbColor::new(0, 0, 0),
            selection_background: RgbColor::new(173, 214, 255),
            selection_foreground: None,
            ansi: AnsiColors {
                black: RgbColor::new(0, 0, 0),
                red: RgbColor::new(205, 49, 49),
                green: RgbColor::new(0, 135, 0),
                yellow: RgbColor::new(205, 120, 0),
                blue: RgbColor::new(0, 0, 205),
                magenta: RgbColor::new(188, 63, 188),
                cyan: RgbColor::new(0, 135, 135),
                white: RgbColor::new(229, 229, 229),
                bright_black: RgbColor::new(102, 102, 102),
                bright_red: RgbColor::new(241, 76, 76),
                bright_green: RgbColor::new(35, 209, 35),
                bright_yellow: RgbColor::new(245, 155, 35),
                bright_blue: RgbColor::new(59, 142, 234),
                bright_magenta: RgbColor::new(214, 112, 214),
                bright_cyan: RgbColor::new(35, 180, 180),
                bright_white: RgbColor::new(255, 255, 255),
            },
        }
    }
}

/// 16 ANSI colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnsiColors {
    pub black: RgbColor,
    pub red: RgbColor,
    pub green: RgbColor,
    pub yellow: RgbColor,
    pub blue: RgbColor,
    pub magenta: RgbColor,
    pub cyan: RgbColor,
    pub white: RgbColor,
    pub bright_black: RgbColor,
    pub bright_red: RgbColor,
    pub bright_green: RgbColor,
    pub bright_yellow: RgbColor,
    pub bright_blue: RgbColor,
    pub bright_magenta: RgbColor,
    pub bright_cyan: RgbColor,
    pub bright_white: RgbColor,
}

impl Default for AnsiColors {
    fn default() -> Self {
        // Standard VGA colors
        Self {
            black: RgbColor::new(0, 0, 0),
            red: RgbColor::new(205, 49, 49),
            green: RgbColor::new(13, 188, 121),
            yellow: RgbColor::new(229, 229, 16),
            blue: RgbColor::new(36, 114, 200),
            magenta: RgbColor::new(188, 63, 188),
            cyan: RgbColor::new(17, 168, 205),
            white: RgbColor::new(229, 229, 229),
            bright_black: RgbColor::new(102, 102, 102),
            bright_red: RgbColor::new(241, 76, 76),
            bright_green: RgbColor::new(35, 209, 139),
            bright_yellow: RgbColor::new(245, 245, 67),
            bright_blue: RgbColor::new(59, 142, 234),
            bright_magenta: RgbColor::new(214, 112, 214),
            bright_cyan: RgbColor::new(41, 184, 219),
            bright_white: RgbColor::new(255, 255, 255),
        }
    }
}

impl AnsiColors {
    /// Get color by index (0-15).
    pub fn get(&self, index: u8) -> RgbColor {
        match index {
            0 => self.black,
            1 => self.red,
            2 => self.green,
            3 => self.yellow,
            4 => self.blue,
            5 => self.magenta,
            6 => self.cyan,
            7 => self.white,
            8 => self.bright_black,
            9 => self.bright_red,
            10 => self.bright_green,
            11 => self.bright_yellow,
            12 => self.bright_blue,
            13 => self.bright_magenta,
            14 => self.bright_cyan,
            15 => self.bright_white,
            _ => self.white,
        }
    }
}

/// RGB color value.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    /// Create a new RGB color.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Create from hex string (with or without #).
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

        Some(Self { r, g, b })
    }

    /// Convert to hex string.
    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Convert to CSS rgb() format.
    pub fn to_css(&self) -> String {
        format!("rgb({}, {}, {})", self.r, self.g, self.b)
    }

    /// Calculate luminance (0.0 - 1.0).
    pub fn luminance(&self) -> f32 {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;
        0.299 * r + 0.587 * g + 0.114 * b
    }

    /// Check if this is a "light" color.
    pub fn is_light(&self) -> bool {
        self.luminance() > 0.5
    }
}

/// Cursor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorConfig {
    /// Cursor style.
    pub style: CursorStyle,
    /// Whether cursor blinks.
    pub blink: bool,
    /// Blink rate in milliseconds.
    pub blink_rate_ms: u32,
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            style: CursorStyle::Block,
            blink: true,
            blink_rate_ms: 530,
        }
    }
}

/// Cursor style.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

/// Scrollback buffer configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollbackConfig {
    /// Maximum number of lines to keep.
    pub lines: usize,
    /// Whether to limit scrollback.
    pub enabled: bool,
}

impl Default for ScrollbackConfig {
    fn default() -> Self {
        Self {
            lines: 10000,
            enabled: true,
        }
    }
}

/// Bell/alert configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BellConfig {
    /// Whether bell is enabled.
    pub enabled: bool,
    /// Whether to use audible bell.
    pub audible: bool,
    /// Whether to use visual bell (flash).
    pub visual: bool,
    /// Visual bell duration in milliseconds.
    pub visual_duration_ms: u32,
}

impl Default for BellConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            audible: false,
            visual: true,
            visual_duration_ms: 100,
        }
    }
}

/// Profile manager for handling multiple terminal profiles.
pub struct ProfileManager {
    profiles: HashMap<String, TerminalProfile>,
    default_profile_id: Option<String>,
}

impl ProfileManager {
    /// Create a new profile manager.
    pub fn new() -> Self {
        let mut manager = Self {
            profiles: HashMap::new(),
            default_profile_id: None,
        };

        // Add default profile
        let default = TerminalProfile::default().as_default();
        manager.default_profile_id = Some(default.id.clone());
        manager.profiles.insert(default.id.clone(), default);

        manager
    }

    /// Add a profile.
    pub fn add(&mut self, profile: TerminalProfile) {
        if profile.is_default {
            self.default_profile_id = Some(profile.id.clone());
        }
        self.profiles.insert(profile.id.clone(), profile);
    }

    /// Remove a profile.
    pub fn remove(&mut self, id: &str) -> Option<TerminalProfile> {
        let removed = self.profiles.remove(id);
        if self.default_profile_id.as_deref() == Some(id) {
            self.default_profile_id = self.profiles.keys().next().cloned();
        }
        removed
    }

    /// Get a profile by ID.
    pub fn get(&self, id: &str) -> Option<&TerminalProfile> {
        self.profiles.get(id)
    }

    /// Get a mutable profile by ID.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut TerminalProfile> {
        self.profiles.get_mut(id)
    }

    /// Get the default profile.
    pub fn default_profile(&self) -> Option<&TerminalProfile> {
        self.default_profile_id
            .as_ref()
            .and_then(|id| self.profiles.get(id))
    }

    /// Set the default profile.
    pub fn set_default(&mut self, id: &str) -> bool {
        if self.profiles.contains_key(id) {
            // Unmark old default
            if let Some(old_id) = &self.default_profile_id {
                if let Some(old) = self.profiles.get_mut(old_id) {
                    old.is_default = false;
                }
            }
            // Mark new default
            if let Some(new) = self.profiles.get_mut(id) {
                new.is_default = true;
            }
            self.default_profile_id = Some(id.to_string());
            true
        } else {
            false
        }
    }

    /// List all profiles.
    pub fn list(&self) -> impl Iterator<Item = &TerminalProfile> {
        self.profiles.values()
    }

    /// Get number of profiles.
    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    /// Check if there are no profiles.
    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    /// Find profiles by tag.
    pub fn find_by_tag(&self, tag: &str) -> Vec<&TerminalProfile> {
        self.profiles
            .values()
            .filter(|p| p.tags.contains(&tag.to_string()))
            .collect()
    }

    /// Load profiles from a JSON file.
    pub fn load(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        let profiles: Vec<TerminalProfile> = serde_json::from_str(&content)?;
        
        for profile in profiles {
            if profile.is_default {
                self.default_profile_id = Some(profile.id.clone());
            }
            self.profiles.insert(profile.id.clone(), profile);
        }
        
        Ok(())
    }

    /// Save profiles to a JSON file.
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let profiles: Vec<_> = self.profiles.values().collect();
        let content = serde_json::to_string_pretty(&profiles)?;
        std::fs::write(path, content)
    }
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_creation() {
        let profile = TerminalProfile::new("custom", "Custom Shell")
            .with_shell(ShellConfig::zsh())
            .with_env("CUSTOM_VAR", "value")
            .with_colors(ColorScheme::dracula());

        assert_eq!(profile.name, "Custom Shell");
        assert!(profile.env.contains_key("CUSTOM_VAR"));
    }

    #[test]
    fn test_color_scheme() {
        let scheme = ColorScheme::one_dark();
        assert_eq!(scheme.name, "One Dark");
    }

    #[test]
    fn test_rgb_from_hex() {
        let color = RgbColor::from_hex("#ff8800").unwrap();
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 136);
        assert_eq!(color.b, 0);
    }

    #[test]
    fn test_profile_manager() {
        let mut manager = ProfileManager::new();
        
        assert!(manager.default_profile().is_some());
        
        manager.add(TerminalProfile::new("dev", "Development"));
        assert_eq!(manager.len(), 2);
        
        manager.set_default("dev");
        assert_eq!(manager.default_profile().unwrap().id, "dev");
    }

    #[test]
    fn test_shell_detect() {
        let shell = ShellConfig::detect();
        assert!(!shell.path.to_string_lossy().is_empty());
    }
}
