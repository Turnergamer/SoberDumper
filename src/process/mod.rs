use std::fs;

pub struct MemRegion {
    pub start: usize,
    pub end: usize,
    pub perms: String,
    pub path: String,
}

pub struct Process {
    pid: i32,
    module_base: usize,
    cached_regions: Vec<MemRegion>,
    mem_file: Option<fs::File>,
}

impl Process {
    pub fn new() -> Self {
        Self { pid: 0, module_base: 0, cached_regions: vec![], mem_file: None }
    }

    pub fn attach(&mut self, name: &str) -> bool {
        let pid = match find_process_by_name(name) {
            Some(pid) => pid,
            None => {
                eprintln!("Failed to find process: {}", name);
                return false;
            }
        };
        self.pid = pid;

        let base = match find_libroblox_base(pid) {
            Some(base) => base,
            None => {
                eprintln!("Failed to find libroblox.so base");
                return false;
            }
        };
        self.module_base = base;
        self.cached_regions = parse_maps(pid);
        let mem_path = format!("/proc/{}/mem", pid);
        let mem = fs::OpenOptions::new().read(true).open(&mem_path);
        match mem {
            Ok(f) => self.mem_file = Some(f),
            Err(e) => {
                eprintln!("Failed to open {}: {}", mem_path, e);
                eprintln!("Try running with sudo (the Main process is owned by root)");
                return false;
            }
        }
        println!("Attached to PID: {}, module base: 0x{:x}", pid, base);
        true
    }

    pub fn pid(&self) -> i32 { self.pid }
    pub fn module_base(&self) -> usize { self.module_base }
    pub fn mem_file(&self) -> &fs::File { self.mem_file.as_ref().unwrap() }

    pub fn get_section(&self, name: &str) -> Option<(usize, usize)> {
        if self.module_base == 0 { return None; }

        let mod_end = self.module_base + 0x10000000;
        let mut matching: Vec<(usize, usize)> = vec![];

        for r in &self.cached_regions {
            if r.start < self.module_base || r.start >= mod_end {
                continue;
            }
            let match_perm = match name {
                ".data" => r.perms == "rw-p" || r.perms == "r--p",
                ".rdata" | ".rodata" => r.perms == "r--p",
                ".text" => r.perms.contains('x'),
                _ => false,
            };
            if match_perm {
                matching.push((r.start, r.end));
            }
        }

        if matching.is_empty() { return None; }

        matching.sort_by_key(|&(s, _)| s);
        let region_start = matching[0].0;
        let mut region_end = matching[0].1;

        for i in 1..matching.len() {
            if matching[i].0 <= region_end + 0x1000 {
                region_end = std::cmp::max(region_end, matching[i].1);
            } else {
                region_end = matching[i].1;
            }
        }

        Some((region_start, region_end - region_start))
    }
}

fn find_process_by_name(name: &str) -> Option<i32> {
    let proc = fs::read_dir("/proc").ok()?;
    for entry in proc {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let ft = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if !ft.is_dir() { continue; }
        let pid: i32 = match entry.file_name().to_str()?.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let comm_path = format!("/proc/{}/comm", pid);
        let comm = match fs::read_to_string(comm_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if comm.trim() == name {
            return Some(pid);
        }
    }
    None
}

fn parse_maps(pid: i32) -> Vec<MemRegion> {
    let maps_path = format!("/proc/{}/maps", pid);
    let content = match fs::read_to_string(&maps_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut regions = vec![];
    for line in content.lines() {
        let parts: Vec<&str> = line.splitn(6, ' ').collect();
        if parts.len() < 5 { continue; }
        let addr_range = parts[0];
        let perms = parts[1].to_string();
        let path = if parts.len() > 5 { parts[5].to_string() } else { String::new() };

        let dash = match addr_range.find('-') {
            Some(d) => d,
            None => continue,
        };
        let start = match usize::from_str_radix(&addr_range[..dash], 16) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let end = match usize::from_str_radix(&addr_range[dash+1..], 16) {
            Ok(e) => e,
            Err(_) => continue,
        };
        regions.push(MemRegion { start, end, perms, path });
    }
    regions
}

fn find_libroblox_base(pid: i32) -> Option<usize> {
    let maps = parse_maps(pid);
    let mut candidates: Vec<(usize, usize, String)> = vec![];

    for r in &maps {
        if !r.path.contains("memfd") && !r.path.contains("(deleted)") {
            continue;
        }
        if !r.perms.contains('x') { continue; }
        let size = r.end - r.start;
        if size > 50 * 1024 * 1024 {
            candidates.push((r.start, size, r.path.clone()));
        }
    }

    if candidates.is_empty() { return None; }

    let maps_content = match fs::read_to_string(format!("/proc/{}/maps", pid)) {
        Ok(c) => c,
        Err(_) => {
            // Fallback to first candidate
            candidates.sort_by_key(|&(addr, _, _)| addr);
            return Some(candidates[0].0);
        }
    };
    for line in maps_content.lines() {
        let parts: Vec<&str> = line.splitn(6, ' ').collect();
        if parts.len() < 5 { continue; }
        let addr_range = parts[0];
        let perms = parts[1];
        let offset_str = parts[2];
        let path = if parts.len() > 5 { parts[5] } else { "" };

        if !path.contains("memfd") && !path.contains("(deleted)") { continue; }
        if !perms.contains('x') { continue; }

        let dash = match addr_range.find('-') {
            Some(d) => d,
            None => continue,
        };
        let start = match usize::from_str_radix(&addr_range[..dash], 16) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let offset = match usize::from_str_radix(offset_str, 16) {
            Ok(o) => o,
            Err(_) => continue,
        };

        let end = match usize::from_str_radix(&addr_range[dash+1..], 16) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let size = end - start;
        if size > 50 * 1024 * 1024 && offset == 0 {
            return Some(start);
        }
    }

    // Fallback: lowest address
    candidates.sort_by_key(|&(addr, _, _)| addr);
    Some(candidates[0].0)
}
