//! TOC-aware reading for Operating_*.md files in Volumes
//! Parses NAV headers and backmatter TOC to enable targeted section reads
//! instead of loading entire 300+ line files.
// NAV: TOC at line 270 | 6 fn |  struct | 2026-02-09

use serde_json::{json, Value};
use std::fs;

pub struct TocEntry {
    pub level: u8,
    pub name: String,
    pub start: usize,
    pub end: usize,
    pub keywords: Vec<String>,
}

/// Check if file is a Volumes Operating file with potential TOC
pub fn is_operating_file(path: &str) -> bool {
    let p = path.replace('/', "\\").to_lowercase();
    (p.contains("volumes") || p.contains("my drive"))
        && p.contains("operating_")
        && p.ends_with(".md")
}

/// Parse NAV header from first few lines to get TOC line number
fn parse_nav_line(content: &str) -> Option<usize> {
    for line in content.lines().take(5) {
        if let Some(idx) = line.find("TOC at line ") {
            let rest = &line[idx + 12..];
            let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return num_str.parse().ok();
        }
    }
    None
}

/// Parse backmatter TOC into structured entries
fn parse_toc(content: &str, toc_start: usize) -> Vec<TocEntry> {
    let lines: Vec<&str> = content.lines().collect();
    let mut entries = Vec::new();
    let start_idx = toc_start.saturating_sub(1);

    for line in lines.iter().skip(start_idx) {
        if line.contains("=== END FILE NAVIGATION") {
            break;
        }

        let trimmed = line.trim();
        if !trimmed.starts_with('#') {
            continue;
        }

        let level = trimmed.chars().take_while(|&c| c == '#').count() as u8;
        let rest = trimmed.trim_start_matches('#').trim();

        // Find the ": start-end" pattern - more robust than splitting on ": "
        // because section names can contain colons
        let mut range_start = None;
        let bytes = rest.as_bytes();
        for i in 0..bytes.len().saturating_sub(3) {
            if bytes[i] == b':' && bytes[i + 1] == b' ' {
                // Check if what follows looks like "digits-digits"
                let after = &rest[i + 2..];
                if after.starts_with(|c: char| c.is_ascii_digit()) {
                    range_start = Some(i);
                    break;
                }
            }
        }

        if let Some(colon_pos) = range_start {
            let name = rest[..colon_pos].trim().to_string();
            let range_part = &rest[colon_pos + 2..];

            // Parse "start-end [keywords]"
            let parts: Vec<&str> = range_part.splitn(2, ' ').collect();
            if let Some(range_str) = parts.first() {
                let range_parts: Vec<&str> = range_str.split('-').collect();
                if range_parts.len() == 2 {
                    if let (Ok(start), Ok(end)) =
                        (range_parts[0].parse::<usize>(), range_parts[1].parse::<usize>())
                    {
                        let keywords = if parts.len() > 1 {
                            let kw = parts[1]
                                .trim_start_matches('[')
                                .trim_end_matches(']');
                            kw.split(',').map(|s| s.trim().to_string()).collect()
                        } else {
                            vec![]
                        };

                        entries.push(TocEntry {
                            level,
                            name,
                            start,
                            end,
                            keywords,
                        });
                    }
                }
            }
        }
    }

    entries
}

/// Find best matching section by query
fn find_section<'a>(entries: &'a [TocEntry], query: &str) -> Option<&'a TocEntry> {
    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();

    // 1. Exact name match
    for entry in entries {
        if entry.name.to_lowercase() == query_lower {
            return Some(entry);
        }
    }

    // 2. Name contains full query
    for entry in entries {
        if entry.name.to_lowercase().contains(&query_lower) {
            return Some(entry);
        }
    }

    // 3. All query words found in keywords
    for entry in entries {
        let kw_str = entry.keywords.join(" ").to_lowercase();
        if query_words.iter().all(|w| kw_str.contains(w)) {
            return Some(entry);
        }
    }

    // 4. Best partial match across name + keywords
    let mut best_score = 0usize;
    let mut best_entry = None;
    for entry in entries {
        let combined = format!(
            "{} {}",
            entry.name.to_lowercase(),
            entry.keywords.join(" ").to_lowercase()
        );
        let score: usize = query_words.iter().filter(|w| combined.contains(*w)).count();
        if score > best_score {
            best_score = score;
            best_entry = Some(entry);
        }
    }

    if best_score > 0 {
        best_entry
    } else {
        None
    }
}

/// Format TOC as compact readable index
fn format_toc(entries: &[TocEntry], path: &str, total_lines: usize) -> String {
    let filename = std::path::Path::new(path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    let top_sections = entries.iter().filter(|e| e.level <= 2).count();

    let mut out = format!(
        "TOC: {} ({} lines, {} sections)\n\n",
        filename, total_lines, top_sections
    );

    for entry in entries {
        let indent = match entry.level {
            1 => "",
            2 => "  ",
            3 => "    ",
            _ => "      ",
        };
        // Show keywords for top-level sections only, max 4
        let kw_display = if entry.level <= 2 && !entry.keywords.is_empty() {
            let top: Vec<&str> = entry
                .keywords
                .iter()
                .filter(|k| k.len() > 2) // skip tiny keywords
                .take(4)
                .map(|s| s.as_str())
                .collect();
            if top.is_empty() {
                String::new()
            } else {
                format!(" [{}]", top.join(", "))
            }
        } else {
            String::new()
        };

        out.push_str(&format!(
            "{}{}: {}-{}{}\n",
            indent, entry.name, entry.start, entry.end, kw_display
        ));
    }

    out
}

/// Main entry point: TOC-aware read for Operating files
/// - No section: returns compact TOC index
/// - With section: finds matching section, returns ±3 line window
pub fn toc_read(path: &str, section: Option<&str>) -> Value {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return json!({"error": format!("Cannot read: {}", e)}),
    };

    let toc_line = match parse_nav_line(&content) {
        Some(l) => l,
        None => return json!({"toc_available": false}),
    };

    let entries = parse_toc(&content, toc_line);
    if entries.is_empty() {
        return json!({"toc_available": false, "note": "NAV header found but no TOC entries parsed"});
    }

    let total_lines = content.lines().count();
    let all_lines: Vec<&str> = content.lines().collect();

    match section {
        Some(query) => {
            if let Some(entry) = find_section(&entries, query) {
                let ctx = 3usize;
                let start = entry.start.saturating_sub(ctx);
                let end = (entry.end + ctx).min(all_lines.len());
                let start_idx = start.saturating_sub(1); // 0-indexed

                let section_content: String = all_lines[start_idx..end]
                    .iter()
                    .enumerate()
                    .map(|(i, l)| format!("{}: {}", start + i, l))
                    .collect::<Vec<_>>()
                    .join("\n");

                json!({
                    "routed_to": "toc_section",
                    "section": entry.name,
                    "lines": format!("{}-{}", entry.start, entry.end),
                    "context_window": format!("{}-{}", start, end),
                    "result": section_content
                })
            } else {
                // No match - return TOC so Claude can pick the right section
                json!({
                    "routed_to": "toc_index",
                    "note": format!("No section matching '{}'. TOC:", query),
                    "result": format_toc(&entries, path, total_lines)
                })
            }
        }
        None => {
            json!({
                "routed_to": "toc_index",
                "total_lines": total_lines,
                "sections": entries.len(),
                "result": format_toc(&entries, path, total_lines)
            })
        }
    }
}

// === FILE NAVIGATION ===
// Generated: 2026-02-09T11:24:15
// Total: 267 lines | 6 functions |  structs | 0 constants
//
// IMPORTS: serde_json, std
//
// FUNCTIONS:
//   pub +is_operating_file: 17-22
//   parse_nav_line: 25-34
//   parse_toc: 37-105 [med]
//   find_section: 108-155
//   format_toc: 158-203
//   pub +toc_read: 208-267 [med]
//
// === END FILE NAVIGATION ===