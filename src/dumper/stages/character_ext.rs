use std::fs::File;
use crate::memory;
use crate::rtti;
use crate::dumper::{G_DUMPER, G_DATA_MODEL_ADDR, collect_children};

pub fn dump(mem: &File) -> bool {
    eprintln!("[character_ext]");

    let dm_addr = unsafe { G_DATA_MODEL_ADDR };
    let lp_off = G_DUMPER.get_offset("Players", "LocalPlayer").unwrap_or(0);
    if lp_off == 0 { eprintln!("  No LocalPlayer offset"); return true; }

    let pa = (|| {
        for range in [0x2000usize, 0x4000, 0x8000] {
            if let Some(off) = rtti::find(mem, dm_addr, "Players@RBX", range, 8) {
                if let Some(pa) = memory::read::<usize>(mem, dm_addr + off) {
                    return Some(pa);
                }
            }
        }
        for off in (0..0x2000).step_by(8) {
            let ptr = memory::read::<usize>(mem, dm_addr + off)?;
            if ptr < 0x10000 { continue; }
            if let Some(r) = rtti::scan_rtti(mem, ptr) {
                if r.name == "Players@RBX" { return Some(ptr); }
            }
        }
        None
    })();

    let pa = match pa {
        Some(p) => p,
        None => { eprintln!("  Players not found"); return true; }
    };

    let lp_addr = match memory::read::<usize>(mem, pa + lp_off) {
        Some(a) => a,
        None => { eprintln!("  LocalPlayer not found"); return true; }
    };

    let char_off = G_DUMPER.get_offset("Player", "Character").unwrap_or(0);
    if char_off == 0 { return true; }
    let char_addr = match memory::read::<usize>(mem, lp_addr + char_off) {
        Some(a) => a,
        None => { eprintln!("  Character not found"); return true; }
    };
    eprintln!("  Character @ 0x{:x}", char_addr);

    // Find Tool in character children
    let kids = collect_children(mem, char_addr);
    for &child in &kids {
        if let Some(r) = rtti::scan_rtti(mem, child) {
            if r.name == "Tool@RBX" {
                eprintln!("  Tool @ 0x{:x}", child);

                // Handle: pointer to Part
                for off in (0..0x200).step_by(8) {
                    let ptr = match memory::read::<usize>(mem, child + off) {
                        Some(p) => p,
                        None => continue,
                    };
                    if ptr >= 0x10000 {
                        if let Some(r2) = rtti::scan_rtti(mem, ptr) {
                            if r2.name.contains("Part") || r2.name == "BasePart@RBX" {
                                G_DUMPER.add_offset("Tool", "Handle", off);
                                eprintln!("  Tool::Handle at +0x{:x}", off);
                                break;
                            }
                        }
                    }
                }

                for off in (0..0x200).step_by(8) {
                    let ptr = match memory::read::<usize>(mem, child + off) {
                        Some(p) => p,
                        None => continue,
                    };
                    if ptr >= 0x10000 {
                        if let Some(s) = memory::read_name_fmt(mem, ptr) {
                            if s.len() >= 2 && s.len() < 100 && s.contains(|c: char| c.is_ascii_alphanumeric()) {
                                if G_DUMPER.get_offset("Tool", "ToolTip").is_none() {
                                    G_DUMPER.add_offset("Tool", "ToolTip", off);
                                    eprintln!("  Tool::ToolTip at +0x{:x}", off);
                                }
                            }
                        }
                    }
                }

                for off in (0..0x100).step_by(1) {
                    let v = match memory::read::<u8>(mem, child + off) {
                        Some(v) => v,
                        None => continue,
                    };
                    if v == 1 {
                        let prev = memory::read::<u8>(mem, child + off.wrapping_sub(1)).unwrap_or(2);
                        let next = memory::read::<u8>(mem, child + off + 1).unwrap_or(2);
                        if prev > 1 && next > 1 {
                            G_DUMPER.add_offset("Tool", "CanBeDropped", off);
                            eprintln!("  Tool::CanBeDropped at +0x{:x}", off);
                            break;
                        }
                    }
                }
            }
        }
    }

    // Find BodyMovers on Character
    for &child in &kids {
        if let Some(r) = rtti::scan_rtti(mem, child) {
            let typ = &r.name[..];
            match typ {
                "BodyVelocity@RBX" | "BodyPosition@RBX" | "BodyGyro@RBX" | "BodyThrust@RBX" => {
                    eprintln!("  {} @ 0x{:x}", r.name, child);

                    if r.name == "BodyVelocity@RBX" {
                        for off in (0..0x100).step_by(4) {
                            let v = match memory::read_f32x3(mem, child + off) {
                                Some(v) => v,
                                None => continue,
                            };
                            if v[0].abs() < 10000.0 && v[1].abs() < 10000.0 && v[2].abs() < 10000.0 {
                                G_DUMPER.add_offset("BodyVelocity", "Velocity", off);
                                eprintln!("  BodyVelocity::Velocity at +0x{:x}", off);
                                break;
                            }
                        }

                        for off in (0..0x100).step_by(4) {
                            let v = match memory::read_f32(mem, child + off) {
                                Some(v) => v,
                                None => continue,
                            };
                            if v > 0.0 && v < 100000.0 {
                                let near_vel = G_DUMPER.get_offset("BodyVelocity", "Velocity").map(|v| off >= v && off <= v + 8).unwrap_or(false);
                                if !near_vel {
                                    G_DUMPER.add_offset("BodyVelocity", "P", off);
                                    eprintln!("  BodyVelocity::P at +0x{:x} ({})", off, v);
                                    break;
                                }
                            }
                        }
                    }

                    if r.name == "BodyPosition@RBX" {
                        for off in (0..0x100).step_by(4) {
                            let v = match memory::read_f32x3(mem, child + off) {
                                Some(v) => v,
                                None => continue,
                            };
                            if v[0].abs() < 100000.0 && v[1].abs() < 100000.0 && v[2].abs() < 100000.0 {
                                G_DUMPER.add_offset("BodyPosition", "Position", off);
                                eprintln!("  BodyPosition::Position at +0x{:x}", off);
                                break;
                            }
                        }
                        for off in (0..0x100).step_by(4) {
                            let v = match memory::read_f32(mem, child + off) {
                                Some(v) => v,
                                None => continue,
                            };
                            if v > 0.0 && v < 100000.0 {
                                let near_pos = G_DUMPER.get_offset("BodyPosition", "Position").map(|p| off >= p && off <= p + 8).unwrap_or(false);
                                if !near_pos {
                                    G_DUMPER.add_offset("BodyPosition", "P", off);
                                    eprintln!("  BodyPosition::P at +0x{:x} ({})", off, v);
                                    break;
                                }
                            }
                        }
                        for off in (0x80..0x200).step_by(4) {
                            let v = match memory::read_f32x3(mem, child + off) {
                                Some(v) => v,
                                None => continue,
                            };
                            if v.iter().all(|&x| x.abs() > 100.0 && x < 1e12) {
                                G_DUMPER.add_offset("BodyPosition", "MaxForce", off);
                                eprintln!("  BodyPosition::MaxForce at +0x{:x}", off);
                                break;
                            }
                        }
                    }

                    if r.name == "BodyGyro@RBX" {
                        for off in (0..0x100).step_by(4) {
                            let buf = memory::read_bytes(mem, child + off, 48);
                            if let Some(b) = buf {
                                let f: &[f32; 12] = unsafe { &*(b.as_ptr() as *const [f32; 12]) };
                                if f.iter().any(|v| v.is_nan() || v.is_infinite()) { continue; }
                                let mut ortho = true;
                                for i in 0..3 {
                                    let len = (f[i*3]*f[i*3] + f[i*3+1]*f[i*3+1] + f[i*3+2]*f[i*3+2]).sqrt();
                                    if (len - 1.0).abs() > 0.02 { ortho = false; break; }
                                }
                                if ortho {
                                    G_DUMPER.add_offset("BodyGyro", "CFrame", off);
                                    eprintln!("  BodyGyro::CFrame at +0x{:x}", off);
                                    break;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // HumanoidDescription via character's Humanoid child
    for &child in &kids {
        if let Some(r) = rtti::scan_rtti(mem, child) {
            if r.name == "Humanoid@RBX" {
                let hkids = collect_children(mem, child);
                for &hk in &hkids {
                    if let Some(r2) = rtti::scan_rtti(mem, hk) {
                        if r2.name == "HumanoidDescription@RBX" {
                            eprintln!("  HumanoidDescription @ 0x{:x}", hk);

                            for off in (0..0x200).step_by(8) {
                                let ptr = match memory::read::<usize>(mem, hk + off) {
                                    Some(p) => p,
                                    None => continue,
                                };
                                if ptr >= 0x10000 {
                                    let first = memory::read::<f32>(mem, ptr).unwrap_or(-1.0);
                                    if first >= 0.0 && first <= 1.0 {
                                        let second = memory::read::<f32>(mem, ptr + 4).unwrap_or(-1.0);
                                        if second >= 0.0 && second <= 1.0 {
                                            G_DUMPER.add_offset("HumanoidDescription", "BodyProportions", off);
                                            eprintln!("  HumanoidDescription::BodyProportions at +0x{:x}", off);
                                            break;
                                        }
                                    }
                                }
                            }

                            for off in (0..0x100).step_by(4) {
                                let v = match memory::read_f32(mem, hk + off) {
                                    Some(v) => v,
                                    None => continue,
                                };
                                if (v - 1.0).abs() < 0.1 && v > 0.0 && v < 10.0 {
                                    if G_DUMPER.get_offset("HumanoidDescription", "HeadScale").is_none() {
                                        G_DUMPER.add_offset("HumanoidDescription", "HeadScale", off);
                                    } else if G_DUMPER.get_offset("HumanoidDescription", "TorsoScale").is_none() {
                                        G_DUMPER.add_offset("HumanoidDescription", "TorsoScale", off);
                                    } else if G_DUMPER.get_offset("HumanoidDescription", "WaistScale").is_none() {
                                        G_DUMPER.add_offset("HumanoidDescription", "WaistScale", off);
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                break;
            }
        }
    }

    true
}
