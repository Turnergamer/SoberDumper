use std::fs::File;
use crate::memory;
use crate::rtti;
use crate::dumper::G_DUMPER;

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
        for off in (0..0x4000).step_by(8) {
            let ptr = match memory::read::<usize>(mem, addr + off) {
                Some(p) => p,
                None => continue,
            };
            if ptr < 0x10000 { continue; }
            if let Some(r) = rtti::scan_rtti(mem, ptr) {
                if r.name == "Humanoid@RBX" {
                    humans.push(ptr);
                    if humans.len() >= 3 { break; }
                }
            }
        }
    }
    humans
}

fn dump_hip_height(mem: &File, humanoids: &[usize]) {
    // HipHeight: float (default ~1.5-2.0, varies by character height)
    for off in (0..0x400).step_by(4) {
        let mut valid = 0;
        let mut varied = false;
        let mut first_val = 0.0f32;
        for (i, &h) in humanoids.iter().enumerate() {
            let v = match memory::read_f32(mem, h + off) {
                Some(v) => v,
                None => break,
            };
            if v.is_nan() || v.is_infinite() || v.is_subnormal() || v < 0.0 || v > 100.0 { break; }
            if i == 0 { first_val = v; }
            if v > 0.5 && v < 10.0 { valid += 1; }
            if i > 0 && (v - first_val).abs() > 0.1 { varied = true; }
        }
        if valid >= 2 && varied {
            G_DUMPER.add_offset("Humanoid", "HipHeight", off);
            eprintln!("  Humanoid::HipHeight at +0x{:x}", off);
            return;
        }
    }
    // Fallback: scan for float 1.5-3.0
    for off in (0..0x400).step_by(4) {
        let v = memory::read_f32(mem, humanoids[0] + off);
        if let Some(v) = v {
            if v > 1.0 && v < 5.0 && !v.is_nan() {
                // Make sure it's not WalkSpeed or JumpPower
                let ws = G_DUMPER.get_offset("Humanoid", "WalkSpeed").unwrap_or(usize::MAX);
                let jp = G_DUMPER.get_offset("Humanoid", "JumpPower").unwrap_or(usize::MAX);
                let jh = G_DUMPER.get_offset("Humanoid", "JumpHeight").unwrap_or(usize::MAX);
                if off != ws && off != jp && off != jh {
                    G_DUMPER.add_offset("Humanoid", "HipHeight", off);
                    eprintln!("  Humanoid::HipHeight at +0x{:x} (fallback)", off);
                    return;
                }
            }
        }
    }
}

fn dump_humanoid_root_part(mem: &File, humanoids: &[usize]) {
    // HumanoidRootPart: pointer to a BasePart (has Primitive@RBX RTTI)
    for off in (0..0x200).step_by(8) {
        let ptr = memory::read::<usize>(mem, humanoids[0] + off);
        if let Some(p) = ptr {
            if p < 0x10000 { continue; }
            if let Some(r) = rtti::scan_rtti(mem, p) {
                if r.name == "Primitive@RBX" {
                    G_DUMPER.add_offset("Humanoid", "HumanoidRootPart", off);
                    eprintln!("  Humanoid::HumanoidRootPart at +0x{:x}", off);
                    return;
                }
            }
            // Also check if the pointed-to object contains a Primitive offset
            let po = G_DUMPER.get_offset("BasePart", "Primitive").unwrap_or(0);
            if po > 0 {
                if let Some(prim) = memory::read::<usize>(mem, p + po) {
                    if prim >= 0x10000 {
                        if let Some(r) = rtti::scan_rtti(mem, prim) {
                            if r.name == "Primitive@RBX" {
                                G_DUMPER.add_offset("Humanoid", "HumanoidRootPart", off);
                                eprintln!("  Humanoid::HumanoidRootPart at +0x{:x} (via Primitive)", off);
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn dump_rig_type(mem: &File, humanoids: &[usize]) {
    // RigType: enum (0=Custom, 1=R6, 2=R15) - stored as u8 or u32
    for off in (0..0x200).step_by(4) {
        let v = memory::read::<u32>(mem, humanoids[0] + off);
        if let Some(v) = v {
            if v <= 2 {
                let next = memory::read::<u32>(mem, humanoids[0] + off + 4).unwrap_or(99);
                if next > 10 {
                    G_DUMPER.add_offset("Humanoid", "RigType", off);
                    eprintln!("  Humanoid::RigType at +0x{:x} ({})", off, v);
                    return;
                }
            }
        }
    }
    for off in (0..0x200).step_by(1) {
        let v = memory::read::<u8>(mem, humanoids[0] + off);
        if let Some(v) = v {
            if v <= 2 {
                let v4 = memory::read::<u32>(mem, humanoids[0] + off - (off % 4)).unwrap_or(0);
                if (v4 & 0xFF) == v as u32 && v4 < 0x1000 {
                    G_DUMPER.add_offset("Humanoid", "RigType", off);
                    eprintln!("  Humanoid::RigType at +0x{:x} (u8)", off);
                    return;
                }
            }
        }
    }
}

fn dump_auto_rotate(mem: &File, humanoids: &[usize]) {
    // AutoRotate: bool (default true = 1)
    for off in (0..0x200).step_by(1) {
        let mut all_one = true;
        for &h in humanoids {
            let v = memory::read::<u8>(mem, h + off);
            match v {
                Some(1) => {},
                _ => { all_one = false; break; }
            }
        }
        if all_one && humanoids.len() >= 2 {
            G_DUMPER.add_offset("Humanoid", "AutoRotate", off);
            eprintln!("  Humanoid::AutoRotate at +0x{:x}", off);
            return;
        }
    }
}

fn dump_platform_stand(mem: &File, humanoids: &[usize]) {
    // PlatformStand: bool (default false = 0)
    for off in (0..0x200).step_by(1) {
        let mut all_zero = true;
        for &h in humanoids {
            let v = memory::read::<u8>(mem, h + off);
            match v {
                Some(0) => {},
                _ => { all_zero = false; break; }
            }
        }
        if all_zero && humanoids.len() >= 2 {
            G_DUMPER.add_offset("Humanoid", "PlatformStand", off);
            eprintln!("  Humanoid::PlatformStand at +0x{:x}", off);
            return;
        }
    }
}

fn dump_seat_part(mem: &File, humanoids: &[usize]) {
    // SeatPart: pointer to BasePart (or null = 0)
    for off in (0..0x200).step_by(8) {
        let ptr = memory::read::<usize>(mem, humanoids[0] + off);
        match ptr {
            Some(0) => {
                // Verify other humanoids also have null (not seated)
                let mut all_null = true;
                for &h in &humanoids[1..] {
                    if let Some(v) = memory::read::<usize>(mem, h + off) {
                        if v != 0 { all_null = false; break; }
                    }
                }
                if all_null {
                    G_DUMPER.add_offset("Humanoid", "SeatPart", off);
                    eprintln!("  Humanoid::SeatPart at +0x{:x} (null)", off);
                    return;
                }
            },
            Some(p) if p >= 0x10000 => {
                if let Some(r) = rtti::scan_rtti(mem, p) {
                    if r.name == "Primitive@RBX" || r.name.contains("Part") || r.name.contains("Seat") {
                        G_DUMPER.add_offset("Humanoid", "SeatPart", off);
                        eprintln!("  Humanoid::SeatPart at +0x{:x}", off);
                        return;
                    }
                }
                let po = G_DUMPER.get_offset("BasePart", "Primitive").unwrap_or(0);
                if po > 0 {
                    if let Some(prim) = memory::read::<usize>(mem, p + po) {
                        if prim >= 0x10000 {
                            if let Some(r) = rtti::scan_rtti(mem, prim) {
                                if r.name == "Primitive@RBX" {
                                    G_DUMPER.add_offset("Humanoid", "SeatPart", off);
                                    eprintln!("  Humanoid::SeatPart at +0x{:x} (via Primitive)", off);
                                    return;
                                }
                            }
                        }
                    }
                }
            },
            _ => {},
        }
    }
}

fn dump_display_distance(mem: &File, humanoids: &[usize]) {
    // DisplayDistanceType: enum (0=Subject, 1=Viewer, 2=None)
    for off in (0..0x200).step_by(4) {
        let v = memory::read::<u32>(mem, humanoids[0] + off);
        if let Some(v) = v {
            if v <= 2 {
                let next = memory::read::<u32>(mem, humanoids[0] + off + 4).unwrap_or(99);
                if next > 10 {
                    G_DUMPER.add_offset("Humanoid", "DisplayDistanceType", off);
                    eprintln!("  Humanoid::DisplayDistanceType at +0x{:x} ({})", off, v);
                    return;
                }
            }
        }
    }
}

fn dump_name_occlusion(mem: &File, humanoids: &[usize]) {
    // NameOcclusion: enum (0=Occlude, 1=AlwaysOnTop, 2=NoOcclusion)
    for off in (0..0x200).step_by(4) {
        let v = memory::read::<u32>(mem, humanoids[0] + off);
        if let Some(v) = v {
            if v <= 2 {
                let next = memory::read::<u32>(mem, humanoids[0] + off + 4).unwrap_or(99);
                if next > 10 {
                    G_DUMPER.add_offset("Humanoid", "NameOcclusion", off);
                    eprintln!("  Humanoid::NameOcclusion at +0x{:x} ({})", off, v);
                    return;
                }
            }
        }
    }
}

fn dump_camera_offset(mem: &File, humanoids: &[usize]) {
    // CameraOffset: Vector3 (default ~(0, 0.5, 0) or similar small offset)
    for off in (0..0x200).step_by(4) {
        let v = memory::read::<[f32; 3]>(mem, humanoids[0] + off);
        if let Some(v) = v {
            if v.iter().any(|x| x.is_nan() || x.is_infinite()) { continue; }
            if v[0].abs() < 0.1 && v[2].abs() < 0.1 && v[1] >= 0.0 && v[1] <= 3.0 {
                G_DUMPER.add_offset("Humanoid", "CameraOffset", off);
                eprintln!("  Humanoid::CameraOffset at +0x{:x}", off);
                return;
            }
        }
    }
}

pub fn dump(mem: &File) -> bool {
    eprintln!("[humanoid_details]");

    let ws_addr = unsafe { crate::dumper::G_WORKSPACE_ADDR };
    let cs = G_DUMPER.get_offset("Instance", "ChildrenStart").unwrap_or(0);
    let ce = G_DUMPER.get_offset("Instance", "ChildrenEnd").unwrap_or(0);

    let humanoids = find_humanoids(mem, ws_addr, cs, ce);
    if humanoids.is_empty() {
        eprintln!("  No humanoids found");
        return true;
    }
    eprintln!("  Found {} humanoid(s)", humanoids.len());

    dump_hip_height(mem, &humanoids);
    dump_humanoid_root_part(mem, &humanoids);
    dump_rig_type(mem, &humanoids);
    dump_auto_rotate(mem, &humanoids);
    dump_platform_stand(mem, &humanoids);
    dump_seat_part(mem, &humanoids);
    dump_display_distance(mem, &humanoids);
    dump_name_occlusion(mem, &humanoids);
    dump_camera_offset(mem, &humanoids);

    true
}
