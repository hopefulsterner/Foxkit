//! # Foxkit Font Ligatures
//!
//! Font ligature detection and rendering support.

use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Font ligatures service
pub struct FontLigaturesService {
    /// Enabled ligature sets
    enabled_sets: RwLock<HashSet<LigatureSet>>,
    /// Custom enabled ligatures
    custom_enabled: RwLock<HashSet<String>>,
    /// Custom disabled ligatures
    custom_disabled: RwLock<HashSet<String>>,
    /// Configuration
    config: RwLock<LigaturesConfig>,
    /// Ligature cache
    cache: RwLock<HashMap<String, Vec<LigatureMatch>>>,
}

impl FontLigaturesService {
    pub fn new() -> Self {
        let mut enabled_sets = HashSet::new();
        enabled_sets.insert(LigatureSet::Arrows);
        enabled_sets.insert(LigatureSet::Equality);
        enabled_sets.insert(LigatureSet::Comparison);

        Self {
            enabled_sets: RwLock::new(enabled_sets),
            custom_enabled: RwLock::new(HashSet::new()),
            custom_disabled: RwLock::new(HashSet::new()),
            config: RwLock::new(LigaturesConfig::default()),
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Configure
    pub fn configure(&self, config: LigaturesConfig) {
        *self.config.write() = config;
        self.cache.write().clear();
    }

    /// Get config
    pub fn config(&self) -> LigaturesConfig {
        self.config.read().clone()
    }

    /// Enable ligature set
    pub fn enable_set(&self, set: LigatureSet) {
        self.enabled_sets.write().insert(set);
        self.cache.write().clear();
    }

    /// Disable ligature set
    pub fn disable_set(&self, set: LigatureSet) {
        self.enabled_sets.write().remove(&set);
        self.cache.write().clear();
    }

    /// Is set enabled
    pub fn is_set_enabled(&self, set: LigatureSet) -> bool {
        self.enabled_sets.read().contains(&set)
    }

    /// Enable specific ligature
    pub fn enable_ligature(&self, ligature: impl Into<String>) {
        let lig = ligature.into();
        self.custom_disabled.write().remove(&lig);
        self.custom_enabled.write().insert(lig);
        self.cache.write().clear();
    }

    /// Disable specific ligature
    pub fn disable_ligature(&self, ligature: impl Into<String>) {
        let lig = ligature.into();
        self.custom_enabled.write().remove(&lig);
        self.custom_disabled.write().insert(lig);
        self.cache.write().clear();
    }

    /// Check if ligature is enabled
    pub fn is_ligature_enabled(&self, ligature: &str) -> bool {
        // Check custom overrides first
        if self.custom_disabled.read().contains(ligature) {
            return false;
        }
        if self.custom_enabled.read().contains(ligature) {
            return true;
        }

        // Check if in enabled set
        let sets = self.enabled_sets.read();
        for set in sets.iter() {
            if set.ligatures().contains(&ligature) {
                return true;
            }
        }

        false
    }

    /// Find ligatures in text
    pub fn find_ligatures(&self, text: &str) -> Vec<LigatureMatch> {
        // Check cache
        if let Some(cached) = self.cache.read().get(text) {
            return cached.clone();
        }

        let mut matches = Vec::new();
        
        // Get all enabled ligatures as owned Strings
        let all_ligatures: Vec<String> = {
            let sets = self.enabled_sets.read();
            let custom = self.custom_enabled.read();
            let disabled = self.custom_disabled.read();

            let mut ligs: Vec<String> = sets.iter()
                .flat_map(|s| s.ligatures())
                .copied()
                .filter(|l| !disabled.contains(*l))
                .map(|s| s.to_string())
                .collect();

            // Add custom enabled
            ligs.extend(custom.iter().cloned());
            ligs
        };

        // Sort by length (longest first) to match greedily
        let mut sorted_ligs = all_ligatures;
        sorted_ligs.sort_by(|a, b| b.len().cmp(&a.len()));

        // Find matches
        let mut i = 0;
        let chars: Vec<char> = text.chars().collect();
        
        while i < chars.len() {
            let remaining: String = chars[i..].iter().collect();
            
            let mut found = false;
            for lig in &sorted_ligs {
                if remaining.starts_with(lig) {
                    matches.push(LigatureMatch {
                        ligature: lig.to_string(),
                        start: i,
                        end: i + lig.chars().count(),
                    });
                    i += lig.chars().count();
                    found = true;
                    break;
                }
            }

            if !found {
                i += 1;
            }
        }

        // Cache results
        self.cache.write().insert(text.to_string(), matches.clone());

        matches
    }

    /// Get all enabled ligatures
    pub fn enabled_ligatures(&self) -> Vec<String> {
        let sets = self.enabled_sets.read();
        let custom = self.custom_enabled.read();
        let disabled = self.custom_disabled.read();

        let mut ligs: Vec<String> = sets.iter()
            .flat_map(|s| s.ligatures())
            .copied()
            .filter(|l| !disabled.contains(*l))
            .map(|s| s.to_string())
            .collect();

        ligs.extend(custom.iter().cloned());
        ligs.sort();
        ligs.dedup();
        ligs
    }

    /// Clear cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }
}

impl Default for FontLigaturesService {
    fn default() -> Self {
        Self::new()
    }
}

/// Ligature match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LigatureMatch {
    /// The ligature string
    pub ligature: String,
    /// Start character index
    pub start: usize,
    /// End character index
    pub end: usize,
}

/// Ligature set
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LigatureSet {
    /// Arrow ligatures (-> => <- etc)
    Arrows,
    /// Equality (== === != !== etc)
    Equality,
    /// Comparison (<= >= etc)
    Comparison,
    /// Logical operators (&& || etc)
    Logical,
    /// Scope operators (:: etc)
    Scope,
    /// HTML/XML (</ /> <!-- etc)
    Markup,
    /// Pipes and compositions (|> <| etc)
    Pipes,
    /// Comments (// /* */ etc)
    Comments,
    /// Misc (... :: www etc)
    Misc,
}

impl LigatureSet {
    pub fn ligatures(&self) -> &'static [&'static str] {
        match self {
            Self::Arrows => &[
                "->", "=>", "<-", "<=", "<->", "<=>",
                "-->", "==>", "<--", "<==",
                "->>", "=>>", "<<-", "<<=",
                "~>", "<~", "~~>", "<~~",
            ],
            Self::Equality => &[
                "==", "===", "!=", "!==",
                "=/=", "=!=", "=/==",
            ],
            Self::Comparison => &[
                "<=", ">=", "<>", "><",
                "<<", ">>", "<<<", ">>>",
            ],
            Self::Logical => &[
                "&&", "||", "!!", "??",
                "?:", "::",
            ],
            Self::Scope => &[
                "::", ":::", "...", "..",
            ],
            Self::Markup => &[
                "</", "/>", "<!--", "-->",
                "<?", "?>", "<!", "<!>",
            ],
            Self::Pipes => &[
                "|>", "<|", "|>>", "<<|",
                "<|>", ">>=", "=<<",
            ],
            Self::Comments => &[
                "//", "///", "/*", "*/",
                "/**", "**/",
            ],
            Self::Misc => &[
                "www", "***", "+++", "---",
                "###", ";;", ";;",
            ],
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Arrows => "Arrow operators",
            Self::Equality => "Equality comparisons",
            Self::Comparison => "Comparison operators",
            Self::Logical => "Logical operators",
            Self::Scope => "Scope and spread operators",
            Self::Markup => "HTML/XML markup",
            Self::Pipes => "Pipe and composition operators",
            Self::Comments => "Comment delimiters",
            Self::Misc => "Miscellaneous ligatures",
        }
    }

    pub fn all() -> &'static [LigatureSet] {
        &[
            Self::Arrows,
            Self::Equality,
            Self::Comparison,
            Self::Logical,
            Self::Scope,
            Self::Markup,
            Self::Pipes,
            Self::Comments,
            Self::Misc,
        ]
    }
}

/// Ligatures configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LigaturesConfig {
    /// Enable ligatures globally
    pub enabled: bool,
    /// Enabled ligature sets
    pub enabled_sets: Vec<LigatureSet>,
    /// Explicitly enabled ligatures
    pub explicit_enable: Vec<String>,
    /// Explicitly disabled ligatures
    pub explicit_disable: Vec<String>,
    /// Disable in specific contexts
    pub disable_in_strings: bool,
    /// Disable in comments
    pub disable_in_comments: bool,
}

impl Default for LigaturesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            enabled_sets: vec![
                LigatureSet::Arrows,
                LigatureSet::Equality,
                LigatureSet::Comparison,
                LigatureSet::Logical,
            ],
            explicit_enable: Vec::new(),
            explicit_disable: Vec::new(),
            disable_in_strings: false,
            disable_in_comments: false,
        }
    }
}

/// Common programming font ligature support info
pub mod fonts {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FontInfo {
        pub name: &'static str,
        pub supports_ligatures: bool,
        pub supported_sets: Vec<LigatureSet>,
    }

    pub fn fira_code() -> FontInfo {
        FontInfo {
            name: "Fira Code",
            supports_ligatures: true,
            supported_sets: LigatureSet::all().to_vec(),
        }
    }

    pub fn jetbrains_mono() -> FontInfo {
        FontInfo {
            name: "JetBrains Mono",
            supports_ligatures: true,
            supported_sets: LigatureSet::all().to_vec(),
        }
    }

    pub fn cascadia_code() -> FontInfo {
        FontInfo {
            name: "Cascadia Code",
            supports_ligatures: true,
            supported_sets: LigatureSet::all().to_vec(),
        }
    }

    pub fn iosevka() -> FontInfo {
        FontInfo {
            name: "Iosevka",
            supports_ligatures: true,
            supported_sets: LigatureSet::all().to_vec(),
        }
    }

    pub fn victor_mono() -> FontInfo {
        FontInfo {
            name: "Victor Mono",
            supports_ligatures: true,
            supported_sets: LigatureSet::all().to_vec(),
        }
    }
}
