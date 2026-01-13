//! Inlay hint providers

use std::path::PathBuf;
use async_trait::async_trait;

use crate::{InlayHint, InlayHintsConfig};

/// Inlay hint provider trait
#[async_trait]
pub trait InlayHintProvider: Send + Sync {
    /// Provider ID
    fn id(&self) -> &str;

    /// Languages this provider supports
    fn languages(&self) -> &[&str];

    /// Provide inlay hints
    async fn provide_hints(
        &self,
        file: &PathBuf,
        content: &str,
        config: &InlayHintsConfig,
    ) -> anyhow::Result<Vec<InlayHint>>;
}

/// Language detection helper
pub fn detect_language(file: &PathBuf) -> Option<&'static str> {
    let ext = file.extension()?.to_str()?;
    
    match ext {
        "rs" => Some("rust"),
        "ts" | "tsx" => Some("typescript"),
        "js" | "jsx" | "mjs" => Some("javascript"),
        "py" => Some("python"),
        "go" => Some("go"),
        "java" => Some("java"),
        "kt" | "kts" => Some("kotlin"),
        "swift" => Some("swift"),
        "cs" => Some("csharp"),
        "cpp" | "cc" | "cxx" => Some("cpp"),
        "c" | "h" => Some("c"),
        "zig" => Some("zig"),
        "ml" | "mli" => Some("ocaml"),
        "hs" => Some("haskell"),
        "scala" => Some("scala"),
        "rb" => Some("ruby"),
        "php" => Some("php"),
        _ => None,
    }
}
