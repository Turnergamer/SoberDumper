use std::fs::File;
use crate::memory;
use crate::rtti;
use crate::dumper::{G_DUMPER, G_DATA_MODEL_ADDR};

fn collect_children(mem: &File, addr: usize, cs: usize, ce: usize) -> Vec<usize> {
    let mut out = vec![];
    let head = memory::read::<usize>(mem, addr + cs).unwrap_or(0);
    if head < 0x10000 { return out; }
    let first = memory::read::<usize>(mem, head).unwrap_or(0);
    let last = memory::read::<usize>(mem, head + ce).unwrap_or(0);
    if first < 0x10000 || last < 0x10000 { return out; }
    let mut node = first;
    for _ in 0..500 {
        if node == last || node == 0 { break; }
        if let Some(child) = memory::read::<usize>(mem, node) {
            if child >= 0x10000 { out.push(child); }
        }
        node += 0x10;
    }
    out
}

pub fn dump(mem: &File) -> bool {
    eprintln!("[player]");

    let dm_addr = unsafe { G_DATA_MODEL_ADDR };
    let cs = G_DUMPER.get_offset("Instance", "ChildrenStart").unwrap_or(0);
    let ce = G_DUMPER.get_offset("Instance", "ChildrenEnd").unwrap_or(0);

    // Try multiple strategies to find Players
    let players_addr = if cs > 0 && ce > 0 {
        let dm_kids = collect_children(mem, dm_addr, cs, ce);
        dm_kids.iter().find_map(|&c| {
            if let Some(r) = rtti::scan_rtti(mem, c) {
                if r.name == "Players@RBX" { Some(c) } else { None }
            } else { None }
        })
    } else {
        None
    };

    // Fallback: wide RTTI scan in DataModel
    let players_addr = players_addr.or_else(|| {
        for range in &[0x2000usize, 0x4000, 0x8000] {
            if let Some(off) = rtti::find(mem, dm_addr, "Players@RBX", *range, 8) {
                if let Some(pa) = memory::read::<usize>(mem, dm_addr + off) {
                    return Some(pa);
                }
            }
        }
        None
    });

    // Fallback: scan DataModel's region for RTTI match directly
    let players_addr = players_addr.or_else(|| {
        for off in (0..0x2000).step_by(8) {
            let ptr = memory::read::<usize>(mem, dm_addr + off)?;
            if ptr < 0x10000 { continue; }
            if let Some(r) = rtti::scan_rtti(mem, ptr) {
                if r.name == "Players@RBX" { return Some(ptr); }
            }
        }
        None
    });

    let pa = match players_addr {
        Some(a) => a,
        None => { eprintln!("  Players not found"); return true; }
    };
    eprintln!("  Players @ 0x{:x}", pa);

    // LocalPlayer via RTTI
    let lp_off = match rtti::find(mem, pa, "Player@RBX", 0x1000, 8) {
        Some(o) => o,
        None => { eprintln!("  LocalPlayer not found"); return true; }
    };
    G_DUMPER.add_offset("Players", "LocalPlayer", lp_off);

    let lp_addr = match memory::read::<usize>(mem, pa + lp_off) {
        Some(a) => a,
        None => { eprintln!("  Failed to read LocalPlayer addr"); return true; }
    };
    eprintln!("  LocalPlayer @ 0x{:x}", lp_addr);

    // Character via RTTI
    for rtti_name in &["ModelInstance@RBX", "Model@RBX"] {
        if let Some(ch) = rtti::find(mem, lp_addr, rtti_name, 0x1000, 8) {
            G_DUMPER.add_offset("Player", "Character", ch);
            eprintln!("  Character at +0x{:x}", ch);
            break;
        }
    }

    // Team via RTTI (pointer to Team@RBX in LocalPlayer)
    if let Some(tm) = rtti::find(mem, lp_addr, "Team@RBX", 0x400, 8) {
        G_DUMPER.add_offset("Player", "Team", tm);
        eprintln!("  Team at +0x{:x}", tm);

        // TeamColor: from the Team object, scan for BrickColor (u8) 0..255
        if let Some(team_addr) = memory::read::<usize>(mem, lp_addr + tm) {
            if team_addr >= 0x10000 {
                for toff in (0..0x100).step_by(1) {
                    let v = match memory::read::<u8>(mem, team_addr + toff) {
                        Some(v) => v,
                        None => continue,
                    };
                    let v32 = memory::read::<u32>(mem, team_addr + toff - (toff % 4)).unwrap_or(0);
                    if (v32 as u8) == v && v32 < 0x10000 {
                        G_DUMPER.add_offset("Team", "TeamColor", toff);
                        eprintln!("  Team::TeamColor at +0x{:x}", toff);
                        break;
                    }
                }
            }
        }
    }

    // UserId: int64 in LocalPlayer (typically > 0)
    for off in (0..0x200).step_by(8) {
        let uid = match memory::read::<i64>(mem, lp_addr + off) {
            Some(uid) => uid,
            None => continue,
        };
        if uid > 0 && uid < 100_000_000_000 {
            let as_usize = match memory::read::<usize>(mem, lp_addr + off) {
                Some(v) => v,
                None => continue,
            };
            if as_usize < 0x10000 || as_usize > 0x7fffffffffff {
                G_DUMPER.add_offset("Player", "UserId", off);
                eprintln!("  UserId at +0x{:x}", off);
                break;
            }
        }
    }

    // DisplayName: scan for player name strings
    for off in (0..0x200).step_by(8) {
        let ptr = match memory::read::<usize>(mem, lp_addr + off) {
            Some(p) => p,
            None => continue,
        };
        if ptr >= 0x10000 {
            if let Some(s) = memory::read_name_fmt(mem, ptr) {
                if s.len() >= 2 && s.len() <= 30 && !s.contains('@') {
                    if G_DUMPER.get_offset("Player", "DisplayName").is_none() {
                        G_DUMPER.add_offset("Player", "DisplayName", off);
                        eprintln!("  DisplayName at +0x{:x} ('{}')", off, s);
                    }
                }
            }
        }
        if G_DUMPER.get_offset("Player", "DisplayName").is_some() { break; }
        if let Some(s) = read_sso(mem, lp_addr + off) {
            if s.len() >= 2 && s.len() <= 30 && !s.contains('@') && s != "Player" {
                G_DUMPER.add_offset("Player", "DisplayName", off);
                eprintln!("  DisplayName at +0x{:x} ('{}')", off, s);
                break;
            }
        }
    }

    // TeamColor on Player directly (BrickColor, u8)
    for off in (0..0x400).step_by(1) {
        let v = match memory::read::<u8>(mem, lp_addr + off) {
            Some(v) => v,
            None => continue,
        };
        if v > 0 && v < 255 {
            let v32 = match memory::read::<u32>(mem, lp_addr + off - (off % 4)) {
                Some(v32) => v32,
                None => continue,
            };
            if (v32 as u8) == v && v32 < 0x1000 {
                G_DUMPER.add_offset("Player", "TeamColor", off);
                eprintln!("  Player::TeamColor at +0x{:x}", off);
                break;
            }
        }
    }

    true
}

fn read_sso(mem: &File, addr: usize) -> Option<String> {
    let size_byte = memory::read::<u8>(mem, addr)?;
    let len = size_byte as usize;
    if len <= 15 {
        let buf = memory::read_bytes(mem, addr + 1, 15)?;
        let end = buf.iter().position(|&b| b == 0).unwrap_or(len);
        let s = String::from_utf8_lossy(&buf[..end]).to_string();
        if s.len() == len { Some(s) } else { None }
    } else {
        let ptr = memory::read::<usize>(mem, addr + 8)?;
        let len2 = memory::read::<usize>(mem, addr + 16)?;
        if ptr < 0x10000 || len2 > 256 { return None; }
        memory::read_string(mem, ptr, len2)
    }
}
