//! Auto-backup before overwrite - silent insurance
//! Keeps last 10 backups per filename, auto-cleanup

use std::fs;
use std::path::Path;
use chrono::Local;

const BACKUP_DIR: &str = r"C:\Backups\auto";
const MAX_BACKUPS_PER_FILE: usize = 10;

/// Backup file if it exists. Silent, no errors propagated.
/// Returns Some(backup_path) if backed up, None if file didn't exist or error.
pub fn backup_if_exists(path: &str) -> Option<String> {
    let source = Path::new(path);
    
    // Only backup if file exists
    if !source.exists() || !source.is_file() {
        return None;
    }
    
    // Create backup dir
    if fs::create_dir_all(BACKUP_DIR).is_err() {
        return None;
    }
    
    // Get filename
    let filename = source.file_name()?.to_str()?;
    
    // Create timestamped backup name
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!("{}_{}", filename, timestamp);
    let backup_path = format!("{}\\{}", BACKUP_DIR, backup_name);
    
    // Copy file
    if fs::copy(source, &backup_path).is_ok() {
        // Cleanup old backups for this filename
        cleanup_old_backups(filename);
        Some(backup_path)
    } else {
        None
    }
}

/// Remove old backups, keep only MAX_BACKUPS_PER_FILE most recent
fn cleanup_old_backups(filename: &str) {
    let backup_dir = Path::new(BACKUP_DIR);
    
    if let Ok(entries) = fs::read_dir(backup_dir) {
        let mut matching: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|n| n.starts_with(filename))
                    .unwrap_or(false)
            })
            .collect();
        
        // Sort by modified time (newest first)
        matching.sort_by(|a, b| {
            let time_a = a.metadata().and_then(|m| m.modified()).ok();
            let time_b = b.metadata().and_then(|m| m.modified()).ok();
            time_b.cmp(&time_a)
        });
        
        // Remove oldest beyond limit
        for old in matching.into_iter().skip(MAX_BACKUPS_PER_FILE) {
            let _ = fs::remove_file(old.path());
        }
    }
}
