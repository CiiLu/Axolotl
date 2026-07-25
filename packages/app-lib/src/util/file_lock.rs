//! Detect processes that hold a file lock on a given path.
//!
//! Uses platform-specific subprocess tools:
//! - Windows: `handle.exe` (Sysinternals) — or empty vec if unavailable
//! - Linux: `fuser -v` with fallback to `lsof -t`
//! - macOS: `lsof -F pcn`
//!
//! All operations gracefully return an empty vec on any error.
//!
//! # Note
//! Code in this module is intentionally dead until called by a later block.
#![allow(dead_code)]

use std::path::Path;

use serde::Serialize;
use tracing::warn;

/// Information about a process that has a file handle open.
#[derive(Debug, Clone, Serialize)]
pub struct LockingProcess {
    pub pid: u32,
    pub name: String,
    pub path: String,
    /// ISO 8601 timestamp of process start time, if available.
    pub start_time: Option<String>,
}

/// Detect processes currently holding the file at `file_path` open.
///
/// Returns an empty vec when no locking processes are found, the platform
/// cannot determine this, required tools are missing, or any error occurs.
pub fn get_locking_processes(file_path: &Path) -> Vec<LockingProcess> {
    let path_str = file_path.to_string_lossy().to_string();

    #[cfg(windows)]
    {
        let procs = get_locking_processes_windows(file_path, &path_str);
        if !procs.is_empty() {
            warn!("Windows file lock detected on {}: {} process(es)", path_str, procs.len());
        }
        procs
    }

    #[cfg(target_os = "linux")]
    {
        let procs = get_locking_processes_linux(file_path, &path_str);
        if !procs.is_empty() {
            warn!("Linux file lock detected on {}: {} process(es)", path_str, procs.len());
        }
        procs
    }

    #[cfg(target_os = "macos")]
    {
        let procs = get_locking_processes_macos(file_path, &path_str);
        if !procs.is_empty() {
            warn!("macOS file lock detected on {}: {} process(es)", path_str, procs.len());
        }
        procs
    }

    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        let _ = path_str;
        Vec::new()
    }
}

// ─── Windows: handle.exe subprocess ────────────────────────────────────────

#[cfg(windows)]
fn get_locking_processes_windows(
    file_path: &Path,
    path_str: &str,
) -> Vec<LockingProcess> {
    let output = run_subprocess("handle.exe", &[
        "-a",
        &file_path.to_string_lossy(),
    ]);

    match output {
        Some(stdout) => parse_handle_exe_output(&stdout, path_str),
        None => {
            // Fallback: try `lsof` if available.
            let out = run_subprocess("lsof", &[
                "-t",
                &file_path.to_string_lossy(),
            ]);
            match out {
                Some(stdout) => {
                    stdout
                        .lines()
                        .filter_map(|l| l.trim().parse::<u32>().ok())
                        .map(|pid| LockingProcess {
                            pid,
                            name: String::new(),
                            path: path_str.to_string(),
                            start_time: None,
                        })
                        .collect()
                }
                None => Vec::new(),
            }
        }
    }
}

/// Parse handle.exe output to extract PID + process name.
///
/// handle.exe output lines look like:
/// ```text
/// chrome.exe        pid: 1234  type: File    <path>
/// ```
#[cfg(windows)]
fn parse_handle_exe_output(output: &str, path_str: &str) -> Vec<LockingProcess> {
    let mut result = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(pid_start) = trimmed.find("pid:") {
            let after_pid = &trimmed[pid_start + 4..];
            let pid_str = after_pid.split_whitespace().next().unwrap_or("");
            if let Ok(pid) = pid_str.parse::<u32>() {
                let name = trimmed[..pid_start].trim().to_string();
                result.push(LockingProcess {
                    pid,
                    name,
                    path: path_str.to_string(),
                    start_time: None,
                });
            }
        }
    }
    result
}

// ─── Linux: fuser / lsof ───────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn get_locking_processes_linux(
    file_path: &Path,
    path_str: &str,
) -> Vec<LockingProcess> {
    let output = run_subprocess("fuser", &["-v", &file_path.to_string_lossy()]);

    let pids: Vec<u32> = match output {
        Some(stderr) => parse_fuser_output(&stderr),
        None => {
            let out = run_subprocess("lsof", &["-t", &file_path.to_string_lossy()]);
            match out {
                Some(stdout) => stdout
                    .lines()
                    .filter_map(|l| l.trim().parse::<u32>().ok())
                    .collect(),
                None => return Vec::new(),
            }
        }
    };

    pids.into_iter()
        .filter_map(|pid| {
            let name = read_proc_name(pid)?;
            let start_time = read_proc_start_time(pid);
            Some(LockingProcess {
                pid,
                name,
                path: path_str.to_string(),
                start_time,
            })
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn parse_fuser_output(stderr: &str) -> Vec<u32> {
    let mut pids = Vec::new();
    for line in stderr.lines() {
        let trimmed = line.trim();
        if let Some(pid_str) = trimmed.split_whitespace().nth(1) {
            if let Ok(pid) = pid_str.parse::<u32>() {
                if pid > 0 {
                    pids.push(pid);
                }
            }
        }
    }
    pids
}

#[cfg(target_os = "linux")]
fn read_proc_name(pid: u32) -> Option<String> {
    let cmdline_path = format!("/proc/{pid}/cmdline");
    let content = std::fs::read_to_string(&cmdline_path).ok()?;
    let first = content.split('\0').next()?;
    if first.is_empty() {
        return None;
    }
    Some(
        std::path::Path::new(first)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| first.to_string()),
    )
}

#[cfg(target_os = "linux")]
fn read_proc_start_time(pid: u32) -> Option<String> {
    let stat_path = format!("/proc/{pid}/stat");
    let content = std::fs::read_to_string(&stat_path).ok()?;
    let paren_end = content.rfind(')')?;
    let after_paren = &content[paren_end + 1..];
    let fields: Vec<&str> = after_paren.split_whitespace().collect();
    // Field 22 (1-indexed) = starttime = index 19 (0-indexed).
    let starttime_str = fields.get(19)?;
    let clock_ticks: u64 = starttime_str.parse().ok()?;

    let clk_tck = 100u64;
    let seconds_since_boot = clock_ticks / clk_tck;

    let stat_content = std::fs::read_to_string("/proc/stat").ok()?;
    let btime_line = stat_content.lines().find(|l| l.starts_with("btime "))?;
    let boot_time_secs: u64 = btime_line.strip_prefix("btime ")?.trim().parse().ok()?;

    let start_time_secs = boot_time_secs + seconds_since_boot;
    let dur = std::time::UNIX_EPOCH + std::time::Duration::from_secs(start_time_secs);
    let datetime = chrono::DateTime::<chrono::Utc>::from(dur);
    Some(datetime.to_rfc3339())
}

// ─── macOS: lsof ───────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn get_locking_processes_macos(
    file_path: &Path,
    path_str: &str,
) -> Vec<LockingProcess> {
    let output = run_subprocess("lsof", &[
        "-F",
        "pcn",
        &file_path.to_string_lossy(),
    ]);

    match output {
        Some(stdout) => parse_lsof_output(&stdout, path_str),
        None => Vec::new(),
    }
}

#[cfg(target_os = "macos")]
fn parse_lsof_output(output: &str, path_str: &str) -> Vec<LockingProcess> {
    let mut result = Vec::new();
    let mut current_pid: Option<u32> = None;
    let mut current_name: Option<String> = None;

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        match line.chars().next() {
            Some('p') => {
                if let (Some(pid), Some(name)) = (current_pid.take(), current_name.take()) {
                    result.push(LockingProcess {
                        pid,
                        name,
                        path: path_str.to_string(),
                        start_time: None,
                    });
                }
                current_pid = line[1..].parse::<u32>().ok();
            }
            Some('c') => {
                current_name = Some(line[1..].to_string());
            }
            Some('n') => {}
            _ => {}
        }
    }

    if let (Some(pid), Some(name)) = (current_pid, current_name) {
        result.push(LockingProcess {
            pid,
            name,
            path: path_str.to_string(),
            start_time: None,
        });
    }

    result
}

// ─── Shared subprocess helper ──────────────────────────────────────────────

/// Run a subprocess with the given arguments, capturing combined output.
/// Returns `None` if the command is not found or fails.
fn run_subprocess(program: &str, args: &[&str]) -> Option<String> {
    let result = std::process::Command::new(program).args(args).output();
    match result {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let combined = if stdout.is_empty() { stderr } else { stdout };
            if combined.is_empty() { None } else { Some(combined) }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_unlocked_file() {
        let dir = tempdir().expect("temp dir");
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").expect("write file");
        let _file = std::fs::File::open(&file_path).expect("open file");
        let processes = get_locking_processes(&file_path);
        assert!(processes.is_empty());
    }

    #[test]
    fn test_nonexistent_file() {
        let path = Path::new("/tmp/this_file_does_not_exist_xyz.test");
        let processes = get_locking_processes(path);
        assert!(processes.is_empty());
    }

    #[test]
    fn test_directory_path() {
        let dir = tempdir().expect("temp dir");
        let processes = get_locking_processes(dir.path());
        let _ = processes;
    }
}
