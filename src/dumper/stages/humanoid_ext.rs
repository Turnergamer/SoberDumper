use std::fs::File;
use crate::memory;
use crate::dumper::{G_DUMPER, G_WORKSPACE_ADDR};

pub fn dump(mem: &File) -> bool {
    eprintln!("[humanoid_ext]");

    let ws_addr = unsafe { G_WORKSPACE_ADDR };
    let cs = G_DUMPER.get_offset("Instance", "ChildrenStart").unwrap_or(0);
    let ce = G_DUMPER.get_offset("Instance", "ChildrenEnd").unwrap_or(0);

    let walk_off = G_DUMPER.get_offset("Humanoid", "WalkSpeed").unwrap_or(0);

    // Find humanoids via RTTI scan in workspace
    let mut humans = vec![];
    for off in (0..0x4000).step_by(8) {
        let ptr = match memory::read::<usize>(mem, ws_addr + off) {
            Some(p) => p,
            None => continue,
        };
        if ptr < 0x10000 { continue; }
        if let Some(r) = crate::rtti::scan_rtti(mem, ptr) {
            if r.name == "Humanoid@RBX" { humans.push(ptr); if humans.len() >= 3 { break; } }
        }
    }
    if humans.is_empty() {
        if cs > 0 && ce > 0 {
            let head = memory::read::<usize>(mem, ws_addr + cs).unwrap_or(0);
            if head >= 0x10000 {
                let first = memory::read::<usize>(mem, head).unwrap_or(0);
                let last = memory::read::<usize>(mem, head + ce).unwrap_or(0);
                if first >= 0x10000 && last >= 0x10000 {
                    let mut node = first;
                    while node != last && node != 0 {
                        if let Some(child) = memory::read::<usize>(mem, node) {
                            if child >= 0x10000 {
                                if let Some(r) = crate::rtti::scan_rtti(mem, child) {
                                    if r.name == "Humanoid@RBX" { humans.push(child); }
                                }
                                let gkids = {
                                    let h = memory::read::<usize>(mem, child + cs).unwrap_or(0);
                                    let f = if h >= 0x10000 { memory::read::<usize>(mem, h).unwrap_or(0) } else { 0 };
                                    let l = if h >= 0x10000 { memory::read::<usize>(mem, h + ce).unwrap_or(0) } else { 0 };
                                    let mut gv = vec![];
                                    if f >= 0x10000 && l >= 0x10000 {
                                        let mut n = f;
                                        while n != l && n != 0 {
                                            if let Some(gc) = memory::read::<usize>(mem, n) {
                                                if gc >= 0x10000 { gv.push(gc); }
                                            }
                                            n += 0x10;
                                        }
                                    }
                                    gv
                                };
                                for gk in &gkids {
                                    if let Some(r) = crate::rtti::scan_rtti(mem, *gk) {
                                        if r.name == "Humanoid@RBX" { humans.push(*gk); }
                                    }
                                }
                            }
                        }
                        node += 0x10;
                        if humans.len() >= 3 { break; }
                    }
                }
            }
        }
    }

    if humans.is_empty() { eprintln!("  No Humanoid found for ext"); return true; }
    eprintln!("  Extended scan on {} Humanoid(s)", humans.len());

    let h = humans[0];

    // HipHeight: float ~0-5 (default depends on rig type)
    for off in (0..0x400).step_by(4) {
        let v = match memory::read_f32(mem, h + off) {
            Some(v) => v,
            None => continue,
        };
        if off == walk_off || off == walk_off + 4 { continue; }
        if v >= 0.0 && v <= 5.0 && !v.is_nan() && !v.is_infinite() && !v.is_subnormal() {
            let mut skip = false;
            for &other in &humans[1..] {
                if let Some(ov) = memory::read_f32(mem, other + off) {
                    if (ov - v).abs() > 0.1 { skip = true; break; }
                }
            }
            if humans.len() <= 1 || !skip {
                G_DUMPER.add_offset("Humanoid", "HipHeight", off);
                eprintln!("  Humanoid::HipHeight at +0x{:x} ({})", off, v);
                break;
            }
        }
    }

    // RigType: u8 enum (0=R15, 1=R6) or u32
    for off in (0..0x400).step_by(1) {
        let v = match memory::read::<u8>(mem, h + off) {
            Some(v) => v,
            None => continue,
        };
        if v <= 1 {
            let mut all_same = true;
            for &other in &humans[1..] {
                if let Some(ov) = memory::read::<u8>(mem, other + off) {
                    if ov != v { all_same = false; break; }
                }
            }
            let prev = memory::read::<u8>(mem, h + off.wrapping_sub(1)).unwrap_or(2);
            let next = memory::read::<u8>(mem, h + off + 1).unwrap_or(2);
            if prev > 1 && next > 1 && (humans.len() <= 1 || !all_same) {
                G_DUMPER.add_offset("Humanoid", "RigType", off);
                eprintln!("  Humanoid::RigType at +0x{:x} (u8={})", off, v);
                break;
            }
        }
    }

    // Sit: bool
    for off in (0..0x400).step_by(1) {
        let v = match memory::read::<u8>(mem, h + off) {
            Some(v) => v,
            None => continue,
        };
        if v == 0 {
            let near_hip = G_DUMPER.get_offset("Humanoid", "HipHeight").map(|h| off >= h && off <= h + 3).unwrap_or(false);
            if !near_hip {
                let prev = memory::read::<u8>(mem, h + off.wrapping_sub(1)).unwrap_or(2);
                let next = memory::read::<u8>(mem, h + off + 1).unwrap_or(2);
                if prev > 1 && next > 1 {
                    G_DUMPER.add_offset("Humanoid", "Sit", off);
                    eprintln!("  Humanoid::Sit at +0x{:x}", off);
                    break;
                }
            }
        }
    }

    // FloorMaterial: u8 enum
    for off in (0..0x400).step_by(1) {
        let v = match memory::read::<u8>(mem, h + off) {
            Some(v) => v,
            None => continue,
        };
        if v >= 1 && v <= 40 {
            let near_rig = G_DUMPER.get_offset("Humanoid", "RigType").map(|r| off == r || off == r + 1).unwrap_or(false);
            let near_sit = G_DUMPER.get_offset("Humanoid", "Sit").map(|s| off == s || off == s + 1).unwrap_or(false);
            if near_rig || near_sit { continue; }
            let prev = memory::read::<u8>(mem, h + off.wrapping_sub(1)).unwrap_or(0);
            let next = memory::read::<u8>(mem, h + off + 1).unwrap_or(0);
            if prev > 40 && next > 40 {
                G_DUMPER.add_offset("Humanoid", "FloorMaterial", off);
                eprintln!("  Humanoid::FloorMaterial at +0x{:x} (u8={})", off, v);
                break;
            }
        }
    }

    // AutoRotate: bool (default true = 1)
    for off in (0..0x400).step_by(1) {
        let v = match memory::read::<u8>(mem, h + off) {
            Some(v) => v,
            None => continue,
        };
        if v == 1 {
            let sit = G_DUMPER.get_offset("Humanoid", "Sit").unwrap_or(usize::MAX);
            let rig = G_DUMPER.get_offset("Humanoid", "RigType").unwrap_or(usize::MAX);
            let floor = G_DUMPER.get_offset("Humanoid", "FloorMaterial").unwrap_or(usize::MAX);
            if off == sit || off == rig || off == floor { continue; }
            let prev = memory::read::<u8>(mem, h + off.wrapping_sub(1)).unwrap_or(2);
            let next = memory::read::<u8>(mem, h + off + 1).unwrap_or(2);
            if prev > 1 && next > 1 {
                G_DUMPER.add_offset("Humanoid", "AutoRotate", off);
                eprintln!("  Humanoid::AutoRotate at +0x{:x}", off);
                break;
            }
        }
    }

    // UseJumpPower: bool (default true = 1)
    for off in (0..0x400).step_by(1) {
        let v = match memory::read::<u8>(mem, h + off) {
            Some(v) => v,
            None => continue,
        };
        if v == 1 {
            let skip = [
                G_DUMPER.get_offset("Humanoid", "Sit").unwrap_or(usize::MAX),
                G_DUMPER.get_offset("Humanoid", "RigType").unwrap_or(usize::MAX),
                G_DUMPER.get_offset("Humanoid", "AutoRotate").unwrap_or(usize::MAX),
                G_DUMPER.get_offset("Humanoid", "FloorMaterial").unwrap_or(usize::MAX),
            ];
            if skip.contains(&off) { continue; }
            let prev = memory::read::<u8>(mem, h + off.wrapping_sub(1)).unwrap_or(2);
            let next = memory::read::<u8>(mem, h + off + 1).unwrap_or(2);
            if prev > 1 && next > 1 {
                G_DUMPER.add_offset("Humanoid", "UseJumpPower", off);
                eprintln!("  Humanoid::UseJumpPower at +0x{:x}", off);
                break;
            }
        }
    }

    true
}
