// Host introspection helpers for the Add Project form.
//
// list_wsl_distros() — `wsl -l --quiet` parsing (Windows host only)
// list_ssh_aliases() — parse `Host` lines from ~/.ssh/config

use std::fs;

pub fn list_wsl_distros() -> Vec<String> {
    #[cfg(windows)]
    {
        // wsl.exe outputs UTF-16 LE with a BOM by default. Read raw bytes.
        let out = std::process::Command::new("wsl").arg("-l").arg("--quiet").output();
        let Ok(out) = out else {
            return vec![];
        };
        if !out.status.success() {
            return vec![];
        }
        let raw = out.stdout;
        // Strip BOM if present, decode UTF-16 LE, fall back to UTF-8 on failure.
        let text = if raw.len() >= 2 && raw[0] == 0xFF && raw[1] == 0xFE {
            let u16s: Vec<u16> = raw[2..]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&u16s)
        } else {
            String::from_utf8_lossy(&raw).into_owned()
        };
        return text
            .lines()
            .map(|l| l.trim().trim_end_matches('\r').to_string())
            .filter(|l| !l.is_empty())
            .collect();
    }
    #[cfg(not(windows))]
    {
        // Not on Windows host — no WSL concept. Return empty.
        vec![]
    }
}

pub fn list_ssh_aliases() -> Vec<String> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return vec![],
    };
    let path = home.join(".ssh").join("config");
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Match `Host alias1 alias2` lines. Skip wildcards (`*`, `!`).
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("host ").or_else(|| lower.strip_prefix("host\t")) {
            // Pull aliases from the original-case line, not the lowercased one.
            let original_rest = line[4..].trim_start();
            for alias in original_rest.split_whitespace() {
                if alias.contains('*') || alias.contains('?') || alias.starts_with('!') {
                    continue;
                }
                if seen.insert(alias.to_string()) {
                    out.push(alias.to_string());
                }
            }
            let _ = rest;
        }
    }
    out
}
