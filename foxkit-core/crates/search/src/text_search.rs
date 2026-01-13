//! Text content search

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::time::Instant;
use crossbeam_channel::Sender;
use ignore::WalkBuilder;
use rayon::prelude::*;

use crate::{
    SearchQuery, SearchOptions, SearchResult, SearchHandle,
    FileMatch, Match, ContextLine,
    result::{SearchStats, SearchError, SearchProgress},
};

/// Text content searcher
pub struct TextSearcher {
    query: SearchQuery,
    options: SearchOptions,
}

impl TextSearcher {
    pub fn new(query: SearchQuery, options: SearchOptions) -> Self {
        Self { query, options }
    }

    pub fn search(&self, paths: Vec<PathBuf>, tx: Sender<SearchResult>, handle: SearchHandle) {
        let start = Instant::now();
        let mut stats = SearchStats::new();

        // Collect files to search
        let mut files = Vec::new();
        
        for path in paths {
            if path.is_file() {
                files.push(path);
            } else if path.is_dir() {
                let walker = WalkBuilder::new(&path)
                    .hidden(!self.options.include_hidden)
                    .follow_links(self.options.follow_symlinks)
                    .ignore(true)
                    .git_ignore(true)
                    .build();

                for entry in walker.flatten() {
                    if entry.path().is_file() {
                        if self.should_search(entry.path()) {
                            files.push(entry.path().to_path_buf());
                        }
                    }
                }
            }
        }

        // Search files in parallel
        let results: Vec<_> = files
            .par_iter()
            .filter_map(|path| {
                if handle.is_cancelled() {
                    return None;
                }
                self.search_file(path)
            })
            .collect();

        // Send results
        for file_match in results {
            if handle.is_cancelled() {
                break;
            }
            
            stats.files_searched += 1;
            if !file_match.matches.is_empty() {
                stats.files_matched += 1;
                stats.total_matches += file_match.matches.len();
                let _ = tx.send(SearchResult::Match(file_match));
            }
            
            if stats.total_matches >= self.options.max_results {
                break;
            }
        }

        stats.duration_ms = start.elapsed().as_millis() as u64;
        let _ = tx.send(SearchResult::Done(stats));
        handle.complete();
    }

    fn should_search(&self, path: &std::path::Path) -> bool {
        // Check file size
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.len() > self.options.max_file_size {
                return false;
            }
        }

        // Check include patterns
        if !self.query.include.is_empty() {
            let matches = self.query.include.iter().any(|pattern| {
                glob_match(pattern, path)
            });
            if !matches {
                return false;
            }
        }

        // Check exclude patterns
        if self.query.exclude.iter().any(|pattern| {
            glob_match(pattern, path)
        }) {
            return false;
        }

        true
    }

    fn search_file(&self, path: &PathBuf) -> Option<FileMatch> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        
        let regex = self.query.to_regex().ok()?;
        let mut file_match = FileMatch::new(path.clone());
        let mut lines: Vec<String> = Vec::new();
        
        // Read all lines for context support
        for line in reader.lines() {
            let line = line.ok()?;
            
            // Check for binary content
            if line.contains('\0') {
                return None; // Binary file, skip
            }
            
            lines.push(line);
        }

        // Search lines
        for (line_num, line) in lines.iter().enumerate() {
            let line_number = line_num + 1;
            
            for mat in regex.find_iter(line) {
                let mut m = Match::new(
                    line_number,
                    mat.start() + 1,
                    mat.end() - mat.start(),
                    line.clone(),
                );

                // Add context
                if self.options.context_before > 0 {
                    let start = line_num.saturating_sub(self.options.context_before);
                    for i in start..line_num {
                        m.context_before.push(ContextLine {
                            line: i + 1,
                            text: lines[i].clone(),
                        });
                    }
                }

                if self.options.context_after > 0 {
                    let end = (line_num + 1 + self.options.context_after).min(lines.len());
                    for i in (line_num + 1)..end {
                        m.context_after.push(ContextLine {
                            line: i + 1,
                            text: lines[i].clone(),
                        });
                    }
                }

                file_match.add_match(m);

                if file_match.count() >= self.options.max_results_per_file {
                    break;
                }
            }

            if file_match.count() >= self.options.max_results_per_file {
                break;
            }
        }

        Some(file_match)
    }
}

/// Simple glob matching
fn glob_match(pattern: &str, path: &std::path::Path) -> bool {
    let path_str = path.to_string_lossy();
    
    if pattern.starts_with("**/") {
        let suffix = &pattern[3..];
        path_str.ends_with(suffix)
    } else if pattern.starts_with("*.") {
        let ext = &pattern[2..];
        path.extension().map(|e| e.to_string_lossy() == ext).unwrap_or(false)
    } else {
        path_str.contains(pattern)
    }
}
