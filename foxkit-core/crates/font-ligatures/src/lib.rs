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
        sorted_ligs.sort_by(|a, b| b.len().