//! Git diff

use std::ops::Range;

/// A diff between two versions
#[derive(Debug, Clone)]
pub struct Diff {
    pub hunks: Vec<DiffHunk>,
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub binary: bool,
}

impl Diff {
    pub fn new() -> Self {
        Self {
            hunks: Vec::new(),
            old_path: None,
            new_path: None,
            binary: false,
        }
    }

    /// Total lines added
    pub fn additions(&self) -> usize {
        self.hunks.iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.kind == DiffLineKind::Addition)
            .count()
    }

    /// Total lines deleted
    pub fn deletions(&self) -> usize {
        self.hunks.iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.kind == DiffLineKind::Deletion)
            .count()
    }
}

impl Default for Diff {
    fn default() -> Self {
        Self::new()
    }
}

/// A diff hunk
#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

impl DiffHunk {
    /// Get range in old file
    pub fn old_range(&self) -> Range<u32> {
        self.old_start..self.old_start + self.old_lines
    }

    /// Get range in new file
    pub fn new_range(&self) -> Range<u32> {
        self.new_start..self.new_start + self.new_lines
    }
}

/// A diff line
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
}

/// Diff line kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Addition,
    Deletion,
    Header,
    Binary,
}

impl DiffLineKind {
    pub fn prefix(&self) -> char {
        match self {
            DiffLineKind::Context => ' ',
            DiffLineKind::Addition => '+',
            DiffLineKind::Deletion => '-',
            DiffLineKind::Header => '@',
            DiffLineKind::Binary => '~',
        }
    }
}

/// Parse unified diff
pub fn parse_unified_diff(diff_text: &str) -> Diff {
    let mut diff = Diff::new();
    let mut current_hunk: Option<DiffHunk> = None;
    let mut old_line = 0u32;
    let mut new_line = 0u32;

    for line in diff_text.lines() {
        if line.starts_with("@@") {
            // Hunk header
            if let Some(hunk) = current_hunk.take() {
                diff.hunks.push(hunk);
            }

            // Parse hunk header: @@ -old_start,old_lines +new_start,new_lines @@
            if let Some(hunk) = parse_hunk_header(line) {
                old_line = hunk.old_start;
                new_line = hunk.new_start;
                current_hunk = Some(hunk);
            }
        } else if let Some(ref mut hunk) = current_hunk {
            let (kind, content) = if line.starts_with('+') {
                (DiffLineKind::Addition, &line[1..])
            } else if line.starts_with('-') {
                (DiffLineKind::Deletion, &line[1..])
            } else if line.starts_with(' ') {
                (DiffLineKind::Context, &line[1..])
            } else {
                (DiffLineKind::Context, line)
            };

            let diff_line = DiffLine {
                kind,
                content: content.to_string(),
                old_line: if kind != DiffLineKind::Addition { 
                    let l = old_line; 
                    old_line += 1; 
                    Some(l) 
                } else { 
                    None 
                },
                new_line: if kind != DiffLineKind::Deletion { 
                    let l = new_line; 
                    new_line += 1; 
                    Some(l) 
                } else { 
                    None 
                },
            };
            hunk.lines.push(diff_line);
        } else if line.starts_with("---") {
            diff.old_path = line.strip_prefix("--- ").map(|s| s.to_string());
        } else if line.starts_with("+++") {
            diff.new_path = line.strip_prefix("+++ ").map(|s| s.to_string());
        }
    }

    if let Some(hunk) = current_hunk {
        diff.hunks.push(hunk);
    }

    diff
}

fn parse_hunk_header(line: &str) -> Option<DiffHunk> {
    // @@ -old_start,old_lines +new_start,new_lines @@ optional header
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }

    let old = parts[1].strip_prefix('-')?;
    let new = parts[2].strip_prefix('+')?;

    let (old_start, old_lines) = parse_range(old)?;
    let (new_start, new_lines) = parse_range(new)?;

    let header = if parts.len() > 4 {
        parts[4..].join(" ")
    } else {
        String::new()
    };

    Some(DiffHunk {
        old_start,
        old_lines,
        new_start,
        new_lines,
        header,
        lines: Vec::new(),
    })
}

fn parse_range(s: &str) -> Option<(u32, u32)> {
    if let Some((start, lines)) = s.split_once(',') {
        Some((start.parse().ok()?, lines.parse().ok()?))
    } else {
        Some((s.parse().ok()?, 1))
    }
}
