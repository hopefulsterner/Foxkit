//! File watcher

use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use notify::{RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher, Config};

/// File watcher
pub struct Watcher {
    _watcher: RecommendedWatcher,
}

impl Watcher {
    pub fn new(path: &Path, tx: mpsc::Sender<WatchEvent>) -> anyhow::Result<Self> {
        let tx_clone = tx.clone();
        
        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, _>| {
            if let Ok(event) = res {
                let kind = match event.kind {
                    notify::EventKind::Create(_) => WatchEventKind::Create,
                    notify::EventKind::Modify(_) => WatchEventKind::Modify,
                    notify::EventKind::Remove(_) => WatchEventKind::Remove,
                    notify::EventKind::Access(_) => WatchEventKind::Access,
                    _ => return,
                };

                for path in event.paths {
                    let _ = tx_clone.blocking_send(WatchEvent {
                        path,
                        kind,
                    });
                }
            }
        })?;

        watcher.watch(path, RecursiveMode::Recursive)?;

        Ok(Self { _watcher: watcher })
    }
}

/// Watch event
#[derive(Debug, Clone)]
pub struct WatchEvent {
    pub path: PathBuf,
    pub kind: WatchEventKind,
}

/// Watch event kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchEventKind {
    Create,
    Modify,
    Remove,
    Access,
}
