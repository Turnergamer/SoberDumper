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

fn find_base_parts(mem: &File, ws_addr: usize, cs: usize, ce: usize) -> Vec<usize> {
    let mut out = vec![];
    if cs > 0 && ce > 0 {
        let ws_kids = collect_children(mem, ws_addr, cs, ce);
        for child in &ws_kids {
            if let Some(r) = rtti::scan_rtti(mem, *child) {
                if r.name == "Part@RBX" || r.name == "WedgePart@RBX" || r.name == "CylinderPart@RBX"
                    || r.name == "CornerWedgePart@RBX" || r.name == "TrussPart@RBX"
                    || r.name == "Seat@RBX" || r.name == "VehicleSeat@RBX"
                    || r.name == "SpawnLocation@RBX" || r.name == "FlagStand@RBX"
                    || r.name == "SkateboardPlatform@RBX" {
                    out.push(*child);
                    if out.len() >= 5 { break; }
                }
            }
            let grandkids = collect_children(mem, *child, cs, ce);
            for gk in &grandkids {
                if let Some(r) = rtti::scan_rtti(mem, *gk) {
                    if r.name == "Part@RBX" || r.name == "WedgePart@RBX" || r.name == "CylinderPart@RBX"
                        || r.name == "CornerWedgePart@RBX" || r.name == "TrussPart@RBX" {
                        out.push(*gk);
                        if out.len() >= 5 { break; }
                    }
                }
            }
            if out.len() >= 5 { break; }
        }
    }
    if out.is_empty() {
        for off in (0..0x4000).step_by(8) {
            let ptr = match memory::read::<usize>(mem, ws_addr + off) {
                Some(p) => p,
                None => continue,
            };
            if ptr < 0x10000 { continue; }
            if let Some(r) = rtti::scan_rtti(mem, ptr) {
                let name = r.name.split('@').next().unwrap_or("");
                if name == "Part" || name == "WedgePart" || name == "CylinderPart" {
                    out.push(ptr);
                    if out.len() >= 5 { break; }
                }
            }
        }
    }
    out
}

fn dump_color3(mem: &File, parts: &[usize]) {
    // Color3: 3 consecutive f32 values (0.0-1.0) that vary across parts
    for off in (0..0x200).step_by(4) {
        let first_c = match memory::read::<[f32; 3]>(mem, parts[0] + off) {
            Some(c) => c,
            None => continue,
        };
        if first_c.iter().any(|x| x.is_nan() || x.is_infinite()) { continue; }
        if !first_c.iter().all(|&x| x >= 0.0 && x <= 1.0) { continue; }
        if first_c[0] + first_c[1] + first_c[2] < 0.01 { continue; }

        // Verify at least one other part has different color
        if parts.len() >= 2 {
            let second_c = memory::read::<[f32; 3]>(mem, parts[1] + off);
            if let Some(sc) = second_c {
                if sc.iter().all(|&x| x >= 0.0 && x <= 1.0) {
                    if (sc[0] - first_c[0]).abs() > 0.01 ||
                       (sc[1] - first_c[1]).abs() > 0.01 ||
                       (sc[2] - first_c[2]).abs() > 0.01 {
                        G_DUMPER.add_offset("BasePart", "Color", off);
                        eprintln!("  BasePart::Color3 at +0x{:x}", off);
                        return;
                    }
                }
            }
        } else {
            G_DUMPER.add_offset("BasePart", "Color", off);
            eprintln!("  BasePart::Color3 at +0x{:x}", off);
            return;
        }
    }
}

fn dump_size_on_bp(mem: &File, parts: &[usize]) {
    // Size on BasePart: vec3 > 0 that varies across parts
    let mut candidates: Vec<(usize, u32)> = vec![];
    for off in (0..0x200).step_by(4) {
        let mut valid = 0;
        for &p in parts {
            let v = match memory::read::<[f32; 3]>(mem, p + off) {
                Some(v) => v,
                None => continue,
            };
            if v.iter().any(|x| x.is_nan() || x.is_infinite() || *x <= 0.0 || *x > 10000.0) {
                continue;
            }
            valid += 1;
        }
        if valid >= 2 {
            candidates.push((off, valid));
        }
    }
    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    if let Some(&(best, _)) = candidates.first() {
        G_DUMPER.add_offset("BasePart", "Size", best);
        eprintln!("  BasePart::Size at +0x{:x}", best);
    }
}

fn dump_material_on_bp(mem: &File, parts: &[usize]) {
    // Material on BasePart: u8 enum (1=Plastic..40+)
    for off in (0..0x200).step_by(1) {
        let mut valid = 0;
        for &p in parts {
            let v = match memory::read::<u8>(mem, p + off) {
                Some(v) => v,
                None => continue,
            };
            if v >= 1 && v <= 45 {
                let v4 = memory::read::<u32>(mem, p + off - (off % 4)).unwrap_or(0);
                if (v4 & 0xFF) == v as u32 && v4 < 0x10000 {
                    valid += 1;
                }
            }
        }
        if valid >= 2 {
            G_DUMPER.add_offset("BasePart", "Material", off);
            eprintln!("  BasePart::Material at +0x{:x}", off);
            return;
        }
    }
}

fn dump_bool_flags(mem: &File, parts: &[usize]) {
    let bool_names = &["Anchored", "CanCollide", "CanQuery", "CanTouch"];
    let ref_off = G_DUMPER.get_offset("BasePart", "CastShadow").unwrap_or(0xED);
    let locked_off = G_DUMPER.get_offset("BasePart", "Locked").unwrap_or(0xEE);
    let mut names_found = 0u32;

    for off in (0..0x200).step_by(1) {
        if names_found == bool_names.len() as u32 { break; }
        if off == ref_off || off == locked_off { continue; }

        let mut vals: Vec<u8> = vec![];
        for &p in parts {
            if let Some(v) = memory::read::<u8>(mem, p + off) { vals.push(v); }
        }
        if vals.len() < 2 { continue; }

        let is_bool_field = vals.iter().all(|&v| v == 0 || v == 1);
        if !is_bool_field { continue; }

        let target_idx = (names_found as usize) % bool_names.len();
        G_DUMPER.add_offset("BasePart", bool_names[target_idx], off);
        eprintln!("  BasePart::{} at +0x{:x}", bool_names[target_idx], off);
        names_found += 1;

        for adj_off in (off + 1..off + 8).step_by(1) {
            if names_found == bool_names.len() as u32 { break; }
            let mut adj_vals: Vec<u8> = vec![];
            for &p in parts {
                if let Some(v) = memory::read::<u8>(mem, p + adj_off) { adj_vals.push(v); }
            }
            if adj_vals.len() >= 2 && adj_vals.iter().all(|&v| v == 0 || v == 1) {
                let target_idx = (names_found as usize) % bool_names.len();
                G_DUMPER.add_offset("BasePart", bool_names[target_idx], adj_off);
                eprintln!("  BasePart::{} at +0x{:x}", bool_names[target_idx], adj_off);
                names_found += 1;
            }
        }
    }
}

fn dump_part_properties(mem: &File, ws_addr: usize, cs: usize, ce: usize) {
    let parts = find_base_parts(mem, ws_addr, cs, ce);
    if parts.is_empty() {
        eprintln!("  No Part instances found for property scan");
        return;
    }
    eprintln!("  Found {} BasePart instance(s) for property scan", parts.len());

    // Reuse existing Primitive offset to verify we're looking at real BaseParts
    let po = G_DUMPER.get_offset("BasePart", "Primitive").unwrap_or(0);
    if po > 0 {
        let mut verified = vec![];
        for &p in &parts {
            if let Some(prim) = memory::read::<usize>(mem, p + po) {
                if prim >= 0x10000 {
                    if let Some(r) = rtti::scan_rtti(mem, prim) {
                        if r.name == "Primitive@RBX" {
                            verified.push(p);
                        }
                    }
                }
            }
        }
        if verified.len() >= 2 {
            dump_color3(mem, &verified);
            dump_size_on_bp(mem, &verified);
            dump_material_on_bp(mem, &verified);
            dump_bool_flags(mem, &verified);
        }
    }

    // Also try without primitive verification
    if G_DUMPER.get_offset("BasePart", "Color").is_none() {
        dump_color3(mem, &parts);
    }
    if G_DUMPER.get_offset("BasePart", "Size").is_none() {
        dump_size_on_bp(mem, &parts);
    }
    if G_DUMPER.get_offset("BasePart", "Material").is_none() {
        dump_material_on_bp(mem, &parts);
    }
    if G_DUMPER.get_offset("BasePart", "Anchored").is_none() {
        dump_bool_flags(mem, &parts);
    }
}

fn dump_primitive_details(mem: &File, ws_addr: usize, cs: usize, ce: usize) {
    let po = G_DUMPER.get_offset("BasePart", "Primitive").unwrap_or(0);
    if po == 0 { return; }

    let mut prim_addrs = vec![];
    if cs > 0 && ce > 0 {
        let ws_kids = collect_children(mem, ws_addr, cs, ce);
        for child in &ws_kids {
            if let Some(prim_ptr) = memory::read::<usize>(mem, *child + po) {
                if prim_ptr >= 0x10000 {
                    if let Some(r) = rtti::scan_rtti(mem, prim_ptr) {
                        if r.name == "Primitive@RBX" {
                            prim_addrs.push(prim_ptr);
                            if prim_addrs.len() >= 3 { break; }
                        }
                    }
                }
            }
        }
    }
    if prim_addrs.is_empty() { return; }

    // Anchored flag directly on Primitive (bit in PrimitiveFlags at 0x80)
    let pf_off = G_DUMPER.get_offset("Primitive", "PrimitiveFlags").unwrap_or(0);
    if pf_off > 0 {
        // PrimitiveFlags bit constants
        G_DUMPER.add_offset("PrimitiveFlags", "Anchored", 0x80);
        G_DUMPER.add_offset("PrimitiveFlags", "CanCollide", 0x01);
        G_DUMPER.add_offset("PrimitiveFlags", "CanTouch", 0x02);
        G_DUMPER.add_offset("PrimitiveFlags", "CanQuery", 0x04);
    }

    // AssemblyLinearVelocity already in baseline, try RotVelocity
    for off in (0x80..0x200).step_by(4) {
        let v = memory::read::<[f32; 3]>(mem, prim_addrs[0] + off);
        if v.is_none() { continue; }
        let v = v.unwrap();
        if v.iter().any(|x| x.is_nan() || x.is_infinite()) { continue; }
        if v[0].abs() > 1000.0 || v[1].abs() > 1000.0 || v[2].abs() > 1000.0 { continue; }

        let lv = G_DUMPER.get_offset("Primitive", "AssemblyLinearVelocity").unwrap_or(usize::MAX);
        let av = G_DUMPER.get_offset("Primitive", "AssemblyAngularVelocity").unwrap_or(usize::MAX);
        if off == lv || off == av { continue; }

        if prim_addrs.len() >= 2 {
            let other = memory::read::<[f32; 3]>(mem, prim_addrs[1] + off);
            if let Some(ov) = other {
                if (ov[0] - v[0]).abs() > 0.01 ||
                   (ov[1] - v[1]).abs() > 0.01 ||
                   (ov[2] - v[2]).abs() > 0.01 {
                    G_DUMPER.add_offset("Primitive", "AssemblyRotVelocity", off);
                    eprintln!("  Primitive::AssemblyRotVelocity at +0x{:x}", off);
                    break;
                }
            }
        }
    }

    // Mass: float (default 0.0 for static, varies for dynamic)
    for off in (0..0x200).step_by(4) {
        let v = memory::read_f32(mem, prim_addrs[0] + off);
        if v.is_none() { continue; }
        let v = v.unwrap();
        if v.is_nan() || v.is_infinite() || v.is_subnormal() || v < 0.0 || v > 1_000_000.0 { continue; }

        if prim_addrs.len() >= 2 {
            let other = memory::read::<f32>(mem, prim_addrs[1] + off);
            if let Some(ov) = other {
                if (ov - v).abs() > 0.1 {
                    G_DUMPER.add_offset("Primitive", "Mass", off);
                    eprintln!("  Primitive::Mass at +0x{:x} ({})", off, v);
                    break;
                }
            }
        }
    }

    // Friction: float (default ~0.3)
    for off in (0..0x200).step_by(4) {
        let mut all_same = true;
        let first = match memory::read_f32(mem, prim_addrs[0] + off) {
            Some(v) => v,
            None => continue,
        };
        if first.is_nan() || first.is_infinite() || first.is_subnormal() || first < 0.0 || first > 1.0 { continue; }
        if (first - 0.3).abs() > 0.29 { continue; }
        for &p in &prim_addrs[1..] {
            if let Some(v) = memory::read::<f32>(mem, p + off) {
                if (v - first).abs() > 0.01 { all_same = false; break; }
            }
        }
        if all_same {
            G_DUMPER.add_offset("Primitive", "Friction", off);
            eprintln!("  Primitive::Friction at +0x{:x} ({})", off, first);
            break;
        }
    }

    // Elasticity: float (default ~0.5)
    for off in (0..0x200).step_by(4) {
        let mut all_same = true;
        let first = match memory::read_f32(mem, prim_addrs[0] + off) {
            Some(v) => v,
            None => continue,
        };
        if first.is_nan() || first.is_infinite() || first.is_subnormal() || first < 0.0 || first > 1.0 { continue; }
        if (first - 0.5).abs() > 0.49 { continue; }
        for &p in &prim_addrs[1..] {
            if let Some(v) = memory::read::<f32>(mem, p + off) {
                if (v - first).abs() > 0.01 { all_same = false; break; }
            }
        }
        if all_same {
            G_DUMPER.add_offset("Primitive", "Elasticity", off);
            eprintln!("  Primitive::Elasticity at +0x{:x} ({})", off, first);
            break;
        }
    }
}

pub fn dump(mem: &File) -> bool {
    eprintln!("[part_details]");

    let ws_addr = unsafe { crate::dumper::G_WORKSPACE_ADDR };
    let cs = G_DUMPER.get_offset("Instance", "ChildrenStart").unwrap_or(0);
    let ce = G_DUMPER.get_offset("Instance", "ChildrenEnd").unwrap_or(0);

    dump_part_properties(mem, ws_addr, cs, ce);
    dump_primitive_details(mem, ws_addr, cs, ce);

    true
}
