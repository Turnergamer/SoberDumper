use std::fs::File;
use crate::memory;
use crate::rtti;
use crate::dumper::{G_DUMPER, G_DATA_MODEL_ADDR, G_WORKSPACE_ADDR};

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
    eprintln!("[lighting]");

    let dm_addr = unsafe { G_DATA_MODEL_ADDR };
    let ws_addr = unsafe { G_WORKSPACE_ADDR };
    let cs = G_DUMPER.get_offset("Instance", "ChildrenStart").unwrap_or(0);
    let ce = G_DUMPER.get_offset("Instance", "ChildrenEnd").unwrap_or(0);

    // Find Lighting via RTTI on DataModel
    let lighting_addr = rtti::find(mem, dm_addr, "Lighting@RBX", 0x1000, 8)
        .and_then(|off| memory::read::<usize>(mem, dm_addr + off))
        .or_else(|| {
            if cs > 0 && ce > 0 {
                let kids = collect_children(mem, dm_addr, cs, ce);
                kids.iter().find_map(|&c| {
                    if let Some(r) = rtti::scan_rtti(mem, c) {
                        if r.name == "Lighting@RBX" { Some(c) } else { None }
                    } else { None }
                })
            } else { None }
        });

    if let Some(lighting) = lighting_addr {
        eprintln!("  Lighting @ 0x{:x}", lighting);

        // Brightness: float (default 1.0)
        for off in (0..0x200).step_by(4) {
            let v = match memory::read_f32(mem, lighting + off) {
                Some(v) => v,
                None => continue,
            };
            if (v - 1.0).abs() < 0.1 && v > 0.0 && v < 10.0 {
                G_DUMPER.add_offset("Lighting", "Brightness", off);
                eprintln!("  Lighting::Brightness at +0x{:x} ({})", off, v);
                break;
            }
        }

        // Atmosphere: find via RTTI in Lighting
        if let Some(atmo_off) = rtti::find(mem, lighting, "Atmosphere@RBX", 0x1000, 8) {
            if let Some(atmo) = memory::read::<usize>(mem, lighting + atmo_off) {
                if atmo >= 0x10000 {
                    eprintln!("  Atmosphere @ 0x{:x}", atmo);

                    // Atmosphere::Color: Color3 (3 f32)
                    for off in (0..0x100).step_by(4) {
                        let v = match memory::read::<[f32; 3]>(mem, atmo + off) {
                            Some(v) => v,
                            None => continue,
                        };
                        if v.iter().any(|x| x.is_nan() || x.is_infinite() || x.is_subnormal()) { continue; }
                        if v.iter().all(|&x| x >= 0.0 && x <= 1.0) && (v[0] + v[1] + v[2]) > 0.1 {
                            G_DUMPER.add_offset("Atmosphere", "Color", off);
                            eprintln!("  Atmosphere::Color at +0x{:x}", off);
                            break;
                        }
                    }

                    // Decay, Glare, Density, Haze, Offset: floats 0.0-1.0
                    for name in &[
                        "Decay",
                        "Glare",
                        "Density",
                        "Haze",
                        "Offset",
                    ] {
                        let color_off = G_DUMPER.get_offset("Atmosphere", "Color");
                        let mut skip_offs = vec![];
                        if let Some(c) = color_off {
                            skip_offs.push(c); skip_offs.push(c + 4); skip_offs.push(c + 8);
                        }
                for prev in &["Decay", "Glare", "Density", "Haze", "Offset"] {
                    if *prev == *name { break; }
                            if let Some(o) = G_DUMPER.get_offset("Atmosphere", prev) {
                                skip_offs.push(o);
                            }
                        }
                        for off in (0..0x100).step_by(4) {
                            let v = match memory::read_f32(mem, atmo + off) {
                                Some(v) => v,
                                None => continue,
                            };
                            if v >= 0.0 && v <= 1.0 && !skip_offs.contains(&off) {
                                G_DUMPER.add_offset("Atmosphere", name, off);

                                eprintln!("  Atmosphere::{} at +0x{:x} ({})", name, off, v);
                                break;
                            }
                        }
                    }
                }
            }
        }

        // BloomEffect: find via RTTI in Lighting
        if let Some(bloom_off) = rtti::find(mem, lighting, "BloomEffect@RBX", 0x1000, 8) {
            if let Some(bloom) = memory::read::<usize>(mem, lighting + bloom_off) {
                if bloom >= 0x10000 {
                    eprintln!("  BloomEffect @ 0x{:x}", bloom);

                    for off in (0..0x200).step_by(4) {
                        let v = match memory::read_f32(mem, bloom + off) {
                            Some(v) => v,
                            None => continue,
                        };
                        if v >= 0.0 && v < 5.0 {
                            G_DUMPER.add_offset("BloomEffect", "Threshold", off);
                            eprintln!("  BloomEffect::Threshold at +0x{:x} ({})", off, v);
                            break;
                        }
                    }
                }
            }
        }
    }

    // Find Terrain via RTTI on Workspace
    let terrain_addr = rtti::find(mem, ws_addr, "Terrain@RBX", 0x1000, 8)
        .and_then(|off| memory::read::<usize>(mem, ws_addr + off))
        .or_else(|| {
            if cs > 0 && ce > 0 {
                let ws_kids = collect_children(mem, ws_addr, cs, ce);
                ws_kids.iter().find_map(|&c| {
                    if let Some(r) = rtti::scan_rtti(mem, c) {
                        if r.name == "Terrain@RBX" { Some(c) } else { None }
                    } else { None }
                })
            } else { None }
        });

    if let Some(terrain) = terrain_addr {
        eprintln!("  Terrain @ 0x{:x}", terrain);

        // WaterColor: Color3
        for off in (0..0x200).step_by(4) {
            let v = match memory::read::<[f32; 3]>(mem, terrain + off) {
                Some(v) => v,
                None => continue,
            };
            if v.iter().any(|x| x.is_nan() || x.is_infinite() || x.is_subnormal()) { continue; }
            if v.iter().all(|&x| x >= 0.0 && x <= 1.0) && (v[0] + v[1] + v[2]) > 0.1 {
                G_DUMPER.add_offset("Terrain", "WaterColor", off);
                eprintln!("  Terrain::WaterColor at +0x{:x}", off);
                break;
            }
        }

        // WaterReflectance, WaterTransparency, GrassLength, WaterWaveSize, WaterWaveSpeed
        for name in &["WaterReflectance", "WaterTransparency", "GrassLength", "WaterWaveSize", "WaterWaveSpeed"] {
            let wc_off = G_DUMPER.get_offset("Terrain", "WaterColor");
            let mut skip_offs = vec![];
            if let Some(w) = wc_off {
                skip_offs.push(w); skip_offs.push(w + 4); skip_offs.push(w + 8);
            }
            for prev in &["WaterReflectance", "WaterTransparency", "GrassLength", "WaterWaveSize", "WaterWaveSpeed"] {
                if *prev == *name { break; }
                if let Some(o) = G_DUMPER.get_offset("Terrain", prev) {
                    skip_offs.push(o);
                }
            }
            for off in (0..0x200).step_by(4) {
                let v = match memory::read_f32(mem, terrain + off) {
                    Some(v) => v,
                    None => continue,
                };
                if v >= 0.0 && v <= 1.0 && !skip_offs.contains(&off) {
                    G_DUMPER.add_offset("Terrain", name, off);
                    eprintln!("  Terrain::{} at +0x{:x} ({})", name, off, v);
                    break;
                }
            }
        }

        // MaterialColors: pointer to array of Color3 values
        for off in (0..0x600).step_by(8) {
            let ptr = match memory::read::<usize>(mem, terrain + off) {
                Some(p) => p,
                None => continue,
            };
            if ptr < 0x10000 { continue; }
            let first_color = match memory::read::<[f32; 3]>(mem, ptr) {
                Some(c) => c,
                None => continue,
            };
            if first_color.iter().any(|x| x.is_nan() || x.is_infinite() || x.is_subnormal()) { continue; }
            if first_color.iter().all(|&x| x >= 0.0 && x <= 1.0) && (first_color[0] + first_color[1] + first_color[2]) > 0.01 {
                let second_color = memory::read::<[f32; 3]>(mem, ptr + 0xC);
                let third_color = memory::read::<[f32; 3]>(mem, ptr + 0x18);
                if second_color.is_some() && third_color.is_some() {
                    G_DUMPER.add_offset("Terrain", "MaterialColors", off);
                    eprintln!("  Terrain::MaterialColors at +0x{:x}", off);
                    break;
                }
            }
        }
    }

    true
}
