use std::fs::File;
use crate::memory;
use crate::rtti;
use crate::dumper::{G_DUMPER, G_WORKSPACE_ADDR};

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

fn find_humanoids_direct(mem: &File, addr: usize) -> Vec<usize> {
    let mut out = vec![];
    for off in (0..0x2000).step_by(8) {
        let ptr = match memory::read::<usize>(mem, addr + off) {
            Some(p) => p,
            None => continue,
        };
        if ptr < 0x10000 { continue; }
        if let Some(r) = rtti::scan_rtti(mem, ptr) {
            if r.name == "Humanoid@RBX" {
                out.push(ptr);
                if out.len() >= 3 { break; }
            }
        }
    }
    out
}

fn find_humanoids(mem: &File, addr: usize, cs: usize, ce: usize) -> Vec<usize> {
    let mut humans = vec![];

    if cs > 0 && ce > 0 {
        let kids = collect_children(mem, addr, cs, ce);
        for child in &kids {
            if let Some(r) = rtti::scan_rtti(mem, *child) {
                if r.name == "Humanoid@RBX" { humans.push(*child); continue; }
            }
            let grandkids = collect_children(mem, *child, cs, ce);
            for gk in &grandkids {
                if let Some(r) = rtti::scan_rtti(mem, *gk) {
                    if r.name == "Humanoid@RBX" { humans.push(*gk); }
                }
            }
        }
    }

    if humans.is_empty() {
        humans = find_humanoids_direct(mem, addr);
    }

    humans
}

pub fn dump(mem: &File) -> bool {
    eprintln!("[humanoid]");

    let ws_addr = unsafe { G_WORKSPACE_ADDR };
    let cs = G_DUMPER.get_offset("Instance", "ChildrenStart").unwrap_or(0);
    let ce = G_DUMPER.get_offset("Instance", "ChildrenEnd").unwrap_or(0);

    let humanoids = find_humanoids(mem, ws_addr, cs, ce);
    if humanoids.is_empty() { eprintln!("  No humanoids found"); return true; }
    eprintln!("  Found {} humanoid(s)", humanoids.len());

    let h = humanoids[0];

    // WalkSpeed: float ≈ 16.0
    for off in (0..0x400).step_by(4) {
        let v = match memory::read_f32(mem, h + off) {
            Some(v) => v,
            None => continue,
        };
        if (v - 16.0).abs() < 1.0 {
            G_DUMPER.add_offset("Humanoid", "WalkSpeed", off);
            for off2 in (off + 8..0x600).step_by(4) {
                let v2 = memory::read_f32(mem, h + off2);
                if let Some(v2) = v2 {
                    if (v2 - 16.0).abs() < 1.0 {
                        G_DUMPER.add_offset("Humanoid", "WalkSpeedCheck", off2);
                        break;
                    }
                }
            }
            break;
        }
    }

    // Health & MaxHealth: find ALL floats ≈ 100
    let mut hundred_floats: Vec<usize> = vec![];
    for off in (0..0x400).step_by(4) {
        let v = match memory::read_f32(mem, h + off) {
            Some(v) => v,
            None => continue,
        };
        if (v - 100.0).abs() < 10.0 {
            hundred_floats.push(off);
            if hundred_floats.len() >= 2 { break; }
        }
    }
    if hundred_floats.len() >= 2 {
        hundred_floats.sort();
        G_DUMPER.add_offset("Humanoid", "MaxHealth", hundred_floats[0]);
        G_DUMPER.add_offset("Humanoid", "Health", hundred_floats[1]);
        eprintln!("  MaxHealth at +0x{:x}, Health at +0x{:x}",
                  hundred_floats[0], hundred_floats[1]);
    } else if hundred_floats.len() == 1 {
        G_DUMPER.add_offset("Humanoid", "Health", hundred_floats[0]);
        eprintln!("  Health at +0x{:x}", hundred_floats[0]);
    }

    // JumpHeight: float ≈ 1.0-2.0 (default ~1.8)
    for off in (0..0x400).step_by(4) {
        let v = match memory::read_f32(mem, h + off) {
            Some(v) => v,
            None => continue,
        };
        if (v - 1.8).abs() < 0.5 && v > 0.5 && v < 5.0 {
            // Verify it's not a dimension/size value by checking it varies across humanoids
            let mut all_same = true;
            for &other in &humanoids[1..] {
                if let Some(ov) = memory::read_f32(mem, other + off) {
                    if (ov - v).abs() > 0.1 { all_same = false; break; }
                }
            }
            if humanoids.len() <= 1 || !all_same {
                G_DUMPER.add_offset("Humanoid", "JumpHeight", off);
                eprintln!("  JumpHeight at +0x{:x}", off);
                break;
            }
        }
    }

    // JumpPower: float ≈ 50 (default)
    for off in (0..0x400).step_by(4) {
        let v = match memory::read_f32(mem, h + off) {
            Some(v) => v,
            None => continue,
        };
        if (v - 50.0).abs() < 5.0 && v > 10.0 {
                G_DUMPER.add_offset("Humanoid", "JumpPower", off);
                eprintln!("  JumpPower at +0x{:x}", off);
            break;
        }
    }

    // MaxSlopeAngle: float ≈ 45-90 (default ~89)
    for off in (0..0x400).step_by(4) {
        let v = match memory::read_f32(mem, h + off) {
            Some(v) => v,
            None => continue,
        };
        if (v - 89.0).abs() < 10.0 && v > 30.0 && v < 100.0 {
                G_DUMPER.add_offset("Humanoid", "MaxSlopeAngle", off);
                eprintln!("  MaxSlopeAngle at +0x{:x}", off);
            break;
        }
    }

    true
}
