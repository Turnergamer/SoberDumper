use std::fs::File;
use crate::memory;
use crate::rtti;
use crate::dumper::{G_DUMPER, G_WORKSPACE_ADDR};

fn collect_children(mem: &File, addr: usize, cs: usize, ce: usize) -> Vec<usize> {
    let mut out = vec![];
    let head = match memory::read::<usize>(mem, addr + cs) {
        Some(h) => h, None => return out,
    };
    let first = match memory::read::<usize>(mem, head) {
        Some(f) => f, None => return out,
    };
    let last = match memory::read::<usize>(mem, head + ce) {
        Some(l) => l, None => return out,
    };
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

fn find_primitive_addrs(mem: &File, addr: usize) -> Vec<usize> {
    let mut out = vec![];
    for off in (0..0x2000).step_by(8) {
        let ptr = match memory::read::<usize>(mem, addr + off) {
            Some(p) => p,
            None => continue,
        };
        if ptr < 0x10000 { continue; }
        if let Some(r) = rtti::scan_rtti(mem, ptr) {
            if r.name == "Primitive@RBX" {
                out.push(ptr);
                if out.len() >= 5 { break; }
            }
        }
    }
    out
}

fn find_primitive_offset(mem: &File, addr: usize, cs: usize, ce: usize) -> Option<usize> {
    let mut offsets: Vec<usize> = vec![];
    if cs > 0 && ce > 0 {
        let ws_children = collect_children(mem, addr, cs, ce);
        for child in &ws_children {
            if let Some(po) = rtti::find(mem, *child, "Primitive@RBX", 0x1000, 8) {
                offsets.push(po);
            }
        }
    }

    if offsets.is_empty() {
        for off in (0..0x200).step_by(8) {
            let ptr = memory::read::<usize>(mem, addr + off)?;
            if ptr < 0x10000 { continue; }
            if let Some(po) = rtti::find(mem, ptr, "Primitive@RBX", 0x1000, 8) {
                offsets.push(po);
                if offsets.len() >= 3 { break; }
            }
        }
    }

    offsets.sort();
    offsets.first().copied()
}

fn collect_parts_and_prims(mem: &File, addr: usize, cs: usize, ce: usize, po: usize) -> Vec<(usize, usize)> {
    let mut out = vec![];
    if cs > 0 && ce > 0 {
        let ws_children = collect_children(mem, addr, cs, ce);
        for child in &ws_children {
            if let Some(pa) = memory::read::<usize>(mem, *child + po) {
                if pa >= 0x10000 {
                    if let Some(r) = rtti::scan_rtti(mem, pa) {
                        if r.name == "Primitive@RBX" {
                            out.push((*child, po));
                            if out.len() >= 3 { break; }
                        }
                    }
                }
            }
        }
    }
    if out.is_empty() {
        let prims = find_primitive_addrs(mem, addr);
        for p in &prims {
            for off in (0..0x200).step_by(8) {
                let ptr = match memory::read::<usize>(mem, addr + off) {
                    Some(p) => p,
                    None => continue,
                };
                if ptr < 0x10000 { continue; }
                if let Some(pa) = memory::read::<usize>(mem, ptr + po) {
                    if pa == *p {
                        out.push((ptr, po));
                        break;
                    }
                }
            }
        }
    }
    out
}

fn dump_base_part_props(mem: &File, bp_addr: usize) {
    // Reflectance: float 0.0-1.0 (default 0.0)
    for off in (0..0x100).step_by(4) {
        let v = match memory::read_f32(mem, bp_addr + off) {
            Some(v) => v,
            None => continue,
        };
        if v.abs() < 0.01 {
            G_DUMPER.add_offset("BasePart", "Reflectance", off);
            eprintln!("  BasePart::Reflectance at +0x{:x}", off);
            break;
        }
    }

    // Transparency: float 0.0-1.0 (default 0.0)
    for off in (0..0x100).step_by(4) {
        let v = match memory::read_f32(mem, bp_addr + off) {
            Some(v) => v,
            None => continue,
        };
        if v.abs() < 0.01 {
            let r_off = G_DUMPER.get_offset("BasePart", "Reflectance").unwrap_or(usize::MAX);
            if off != r_off {
                G_DUMPER.add_offset("BasePart", "Transparency", off);
                eprintln!("  BasePart::Transparency at +0x{:x}", off);
                break;
            }
        }
    }

    // Shape: enum (Block=0, Cylinder=1, Ball=2, Wedge=3, CornerWedge=4)
    for off in (0..0x200).step_by(1) {
        let v = match memory::read::<u8>(mem, bp_addr + off) {
            Some(v) => v,
            None => continue,
        };
        if v <= 4 {
            let v4 = match memory::read::<u32>(mem, bp_addr + off - (off % 4)) {
                Some(v4) => v4,
                None => continue,
            };
            if (v4 & 0xFF) == v as u32 && v4 < 0x1000 {
                G_DUMPER.add_offset("BasePart", "Shape", off);
                eprintln!("  BasePart::Shape at +0x{:x}", off);
                break;
            }
        }
    }

    // CastShadow: bool (default true = 1)
    for off in (0..0x100).step_by(1) {
        let v = match memory::read::<u8>(mem, bp_addr + off) {
            Some(v) => v,
            None => continue,
        };
        if v == 1 {
            G_DUMPER.add_offset("BasePart", "CastShadow", off);
            eprintln!("  BasePart::CastShadow at +0x{:x}", off);
            break;
        }
    }

    // Locked: bool (default false = 0), adjacent to CastShadow
    for off in (0..0x100).step_by(1) {
        let v = match memory::read::<u8>(mem, bp_addr + off) {
            Some(v) => v,
            None => continue,
        };
        if v == 0 {
            let cast = G_DUMPER.get_offset("BasePart", "CastShadow").unwrap_or(usize::MAX);
            if off != cast && (off as isize - cast as isize).abs() <= 4 {
                G_DUMPER.add_offset("BasePart", "Locked", off);
                eprintln!("  BasePart::Locked at +0x{:x}", off);
                break;
            }
        }
    }

    // Massless: bool (default false = 0), near Locked
    for off in (0..0x100).step_by(1) {
        let v = match memory::read::<u8>(mem, bp_addr + off) {
            Some(v) => v,
            None => continue,
        };
        if v == 0 {
            let locked = G_DUMPER.get_offset("BasePart", "Locked").unwrap_or(usize::MAX);
            let cast = G_DUMPER.get_offset("BasePart", "CastShadow").unwrap_or(usize::MAX);
            if off != locked && off != cast
                && ((off as isize - locked as isize).abs() <= 8 || (off as isize - cast as isize).abs() <= 8)
            {
                G_DUMPER.add_offset("BasePart", "Massless", off);
                eprintln!("  BasePart::Massless at +0x{:x}", off);
                break;
            }
        }
    }
}

fn dump_primitive_props(mem: &File, prim_addrs: &[usize]) {
    if prim_addrs.is_empty() { return; }
    let base = prim_addrs[0];

    // AssemblyLinearVelocity: vec3 near CFrame/Position
    for off in (0x80..0x180).step_by(4) {
        let v = match memory::read::<[f32; 3]>(mem, base + off) {
            Some(v) => v,
            None => continue,
        };
        if v.iter().any(|x| x.is_nan() || x.is_infinite()) { continue; }
        if v[0].abs() > 1000.0 || v[1].abs() > 1000.0 || v[2].abs() > 1000.0 { continue; }

        if prim_addrs.len() >= 2 {
            let mut varies = false;
            for &other in &prim_addrs[1..] {
                if let Some(ov) = memory::read::<[f32; 3]>(mem, other + off) {
                    if (ov[0] - v[0]).abs() > 0.01 ||
                       (ov[1] - v[1]).abs() > 0.01 ||
                       (ov[2] - v[2]).abs() > 0.01 {
                        varies = true;
                        break;
                    }
                }
            }
            if varies {
                G_DUMPER.add_offset("Primitive", "AssemblyLinearVelocity", off);
                eprintln!("  Primitive::AssemblyLinearVelocity at +0x{:x}", off);
                break;
            }
        }
    }

    // AssemblyAngularVelocity: vec3 (usually small values)
    for off in (0x80..0x180).step_by(4) {
        let v = match memory::read::<[f32; 3]>(mem, base + off) {
            Some(v) => v,
            None => continue,
        };
        if v.iter().any(|x| x.is_nan() || x.is_infinite()) { continue; }
        if v[0].abs() > 100.0 || v[1].abs() > 100.0 || v[2].abs() > 100.0 { continue; }

        let lv_off = G_DUMPER.get_offset("Primitive", "AssemblyLinearVelocity").unwrap_or(usize::MAX);
        if off == lv_off { continue; }

        if prim_addrs.len() >= 2 {
            let mut varies = false;
            for &other in &prim_addrs[1..] {
                if let Some(ov) = memory::read::<[f32; 3]>(mem, other + off) {
                    if (ov[0] - v[0]).abs() > 0.01 ||
                       (ov[1] - v[1]).abs() > 0.01 ||
                       (ov[2] - v[2]).abs() > 0.01 {
                        varies = true;
                        break;
                    }
                }
            }
            if varies {
                G_DUMPER.add_offset("Primitive", "AssemblyAngularVelocity", off);
                eprintln!("  Primitive::AssemblyAngularVelocity at +0x{:x}", off);
                break;
            }
        }
    }

    // Material: u8 enum (1=Plastic, 2=Wood, etc.)
    for off in (0..0x200).step_by(1) {
        let v = match memory::read::<u8>(mem, base + off) {
            Some(v) => v,
            None => continue,
        };
        if v >= 1 && v <= 40 {
            let v4 = match memory::read::<u32>(mem, base + off - (off % 4)) {
                Some(v4) => v4,
                None => continue,
            };
            if (v4 & 0xFF) == v as u32 && v4 < 0x10000 {
                G_DUMPER.add_offset("Primitive", "Material", off);
                eprintln!("  Primitive::Material at +0x{:x}", off);
                break;
            }
        }
    }

    // PrimitiveFlags: u8 bitfield
    // ANCHORED=0x80, CAN_COLLIDE=0x01, CAN_QUERY=0x04, CAN_TOUCH=0x02
    // Most anchored parts have flags = 0x81 (anchored + can collide)
    for off in (0..0x200).step_by(1) {
        let v = match memory::read::<u8>(mem, base + off) {
            Some(v) => v,
            None => continue,
        };
        if v == 0x81 || v == 0x01 || v == 0x83 || v == 0x85 || v == 0x87 {
            let v4 = match memory::read::<u32>(mem, base + off - (off % 4)) {
                Some(v4) => v4,
                None => continue,
            };
            if (v4 & 0xFF) == v as u32 && v4 < 0x1000 {
                G_DUMPER.add_offset("Primitive", "PrimitiveFlags", off);
                eprintln!("  Primitive::PrimitiveFlags at +0x{:x}", off);
                break;
            }
        }
    }
}

pub fn dump(mem: &File) -> bool {
    eprintln!("[base_part/primitive]");

    let ws_addr = unsafe { G_WORKSPACE_ADDR };
    let cs = G_DUMPER.get_offset("Instance", "ChildrenStart").unwrap_or(0);
    let ce = G_DUMPER.get_offset("Instance", "ChildrenEnd").unwrap_or(0);

    let po = match find_primitive_offset(mem, ws_addr, cs, ce) {
        Some(p) => p,
        None => { eprintln!("  No BasePart/Primitive found"); return true; }
    };
    G_DUMPER.add_offset("BasePart", "Primitive", po);
    eprintln!("  BasePart::Primitive at +0x{:x}", po);

    let parts = collect_parts_and_prims(mem, ws_addr, cs, ce, po);
    let mut prim_addrs: Vec<usize> = vec![];
    for &(bp_addr, _) in &parts {
        if let Some(pa) = memory::read::<usize>(mem, bp_addr + po) {
            if pa >= 0x10000 { prim_addrs.push(pa); }
        }
    }
    if prim_addrs.is_empty() {
        let direct = find_primitive_addrs(mem, ws_addr);
        for p in direct { prim_addrs.push(p); }
    }

    // Primitive::Position and CFrame
    if let Some(&base) = prim_addrs.first() {
        let mut pos_candidates: Vec<usize> = vec![];
        for off in (0x80..0x200).step_by(4) {
            let v = match memory::read::<[f32; 3]>(mem, base + off) {
                Some(v) => v,
                None => continue,
            };
            if v.iter().any(|x| x.is_nan() || x.is_infinite()) { continue; }
            if v[0].abs() < 1e8 && v[1].abs() < 1e8 && v[2].abs() < 1e8 &&
               (v[0].abs() > 1.0 || v[1].abs() > 1.0 || v[2].abs() > 1.0) {
                if off >= 36 {
                    let rot_start = off - 36;
                    let buf = memory::read::<[f32; 9]>(mem, base + rot_start);
                    if let Some(r) = buf {
                        if r.iter().all(|x| !x.is_nan() && !x.is_infinite()) {
                            let mut ortho = true;
                            for i in 0..3 {
                                let a0 = r[i*3];
                                let a1 = r[i*3+1];
                                let a2 = r[i*3+2];
                                let len = (a0*a0 + a1*a1 + a2*a2).sqrt();
                                if (len - 1.0).abs() > 0.02 { ortho = false; break; }
                                if a0.abs() > 1.5 || a1.abs() > 1.5 || a2.abs() > 1.5 { ortho = false; break; }
                            }
                            if ortho {
                                let dot01 = r[0]*r[3] + r[1]*r[4] + r[2]*r[5];
                                let dot02 = r[0]*r[6] + r[1]*r[7] + r[2]*r[8];
                                if dot01.abs() < 0.02 && dot02.abs() < 0.02 {
                                    let det = r[0]*(r[4]*r[8]-r[5]*r[7])
                                            - r[3]*(r[1]*r[8]-r[2]*r[7])
                                            + r[6]*(r[1]*r[5]-r[2]*r[4]);
                                    if (det - 1.0).abs() < 0.02 {
                                        pos_candidates.push(off);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if !pos_candidates.is_empty() {
            let best_pos = pos_candidates[0];
            G_DUMPER.add_offset("Primitive", "Position", best_pos);
            G_DUMPER.add_offset("Primitive", "CFrame", best_pos - 36);
            G_DUMPER.add_offset("Primitive", "Rotation", best_pos - 36);
            G_DUMPER.add_offset("Primitive", "Orientation", best_pos - 36);
            eprintln!("  Primitive::Position at +0x{:x}, CFrame at +0x{:x}",
                      best_pos, best_pos - 36);

            // Primitive::Size
            let co = best_pos - 36;
            let start = co + 0x80;
            let end = co + 0x110;
            let mut size_candidates: Vec<(usize, f32)> = vec![];
            {
                let mut off = start;
                while off < end {
                    let v = match memory::read::<[f32; 3]>(mem, base + off) {
                        Some(v) => v,
                        None => { off += 4; continue; }
                    };
                    if v.iter().any(|x| x.is_nan() || x.is_infinite() || *x <= 0.0 || *x > 1000.0) {
                        off += 4; continue;
                    }
                    if (v[0] - 1.0).abs() < 0.01 && (v[1] - 1.0).abs() < 0.01 && (v[2] - 1.0).abs() < 0.01 {
                        off += 4; continue;
                    }
                    let mut diff_count = 0;
                    let mut total_pairs = 0;
                    for i in 0..prim_addrs.len() {
                        for j in i+1..prim_addrs.len() {
                            total_pairs += 1;
                            let vi = memory::read::<[f32; 3]>(mem, prim_addrs[i] + off);
                            let vj = memory::read::<[f32; 3]>(mem, prim_addrs[j] + off);
                            if let (Some(vi), Some(vj)) = (vi, vj) {
                                if (vi[0] - vj[0]).abs() > 0.01 ||
                                   (vi[1] - vj[1]).abs() > 0.01 ||
                                   (vi[2] - vj[2]).abs() > 0.01 {
                                    diff_count += 1;
                                }
                            }
                        }
                    }
                    let score = if total_pairs > 0 { diff_count as f32 / total_pairs as f32 } else { 1.0 };
                    if score > 0.5 || total_pairs == 0 {
                        size_candidates.push((off, score));
                    }
                    off += 4;
                }
            }

            if !size_candidates.is_empty() {
                size_candidates.sort_by(|a, b| {
                    b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                        .then(b.0.cmp(&a.0))
                });
                let sz = size_candidates[0].0;
                G_DUMPER.add_offset("Primitive", "Size", sz);
                eprintln!("  Primitive::Size at +0x{:x}", sz);
            }
        }

        // Additional Primitive properties
        dump_primitive_props(mem, &prim_addrs);
    }

    // BasePart properties from the first part
    if let Some(&(bp_addr, _)) = parts.first() {
        dump_base_part_props(mem, bp_addr);
    }

    true
}
