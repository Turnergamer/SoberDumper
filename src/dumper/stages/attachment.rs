use std::fs::File;
use crate::memory;
use crate::dumper::{G_DUMPER, G_WORKSPACE_ADDR, find_instances};

pub fn dump(mem: &File) -> bool {
    eprintln!("[attachment]");

    let ws_addr = unsafe { G_WORKSPACE_ADDR };

    let mut atts = find_instances(mem, ws_addr, "Attachment@RBX", 3);
    if atts.is_empty() {
        for off in (0..0x4000).step_by(8) {
            if let Some(ptr) = memory::read::<usize>(mem, ws_addr + off) {
                if ptr >= 0x10000 {
                    if let Some(r) = crate::rtti::scan_rtti(mem, ptr) {
                        if r.name == "Attachment@RBX" { atts.push(ptr); if atts.len() >= 3 { break; } }
                    }
                }
            }
        }
    }
    if atts.is_empty() { eprintln!("  No Attachment found"); return true; }
    eprintln!("  Found {} Attachment(s)", atts.len());

    let a = atts[0];

    for off in (0x0..0x80).step_by(4) {
        let v = match memory::read::<[f32; 3]>(mem, a + off) {
            Some(v) => v,
            None => continue,
        };
        if v.iter().any(|x| x.is_nan() || x.is_infinite()) { continue; }
        if v[0].abs() < 1e6 && v[1].abs() < 1e6 && v[2].abs() < 1e6 {
            if off < 36 { continue; }
            let rot_off = off - 36;
            if rot_off < off && rot_off > 0 {
                let rot_buf = memory::read::<[f32; 9]>(mem, a + rot_off);
                if let Some(r) = rot_buf {
                    if r.iter().all(|x| !x.is_nan() && !x.is_infinite()) {
                        let mut ortho = true;
                        for i in 0..3 {
                            let len = (r[i*3]*r[i*3] + r[i*3+1]*r[i*3+1] + r[i*3+2]*r[i*3+2]).sqrt();
                            if (len - 1.0).abs() > 0.02 { ortho = false; break; }
                        }
                        if ortho {
                            G_DUMPER.add_offset("Attachment", "CFrame", rot_off);
                            G_DUMPER.add_offset("Attachment", "Position", off);
                            eprintln!("  Attachment::CFrame at +0x{:x}, Position at +0x{:x}", rot_off, off);
                            break;
                        }
                    }
                }
            }
        }
    }

    for off in (0..0x100).step_by(4) {
        let v = match memory::read::<[f32; 3]>(mem, a + off) {
            Some(v) => v,
            None => continue,
        };
        if v.iter().any(|x| x.is_nan() || x.is_infinite()) { continue; }
        let len = (v[0]*v[0] + v[1]*v[1] + v[2]*v[2]).sqrt();
        if (len - 1.0).abs() < 0.02 && v[0].abs() <= 1.0 && v[1].abs() <= 1.0 && v[2].abs() <= 1.0 {
            let pos_off = G_DUMPER.get_offset("Attachment", "Position").unwrap_or(usize::MAX);
            let cframe = G_DUMPER.get_offset("Attachment", "CFrame").unwrap_or(usize::MAX);
            let near_cframe = cframe != usize::MAX && (off == cframe || off == cframe.wrapping_add(4) || off == cframe.wrapping_add(8));
            if off != pos_off && !near_cframe {
                G_DUMPER.add_offset("Attachment", "Axis", off);
                eprintln!("  Attachment::Axis at +0x{:x}", off);
                for off2 in (off + 12..off + 24).step_by(4) {
                    let v2 = match memory::read::<[f32; 3]>(mem, a + off2) {
                        Some(v2) => v2,
                        None => continue,
                    };
                    let len2 = (v2[0]*v2[0] + v2[1]*v2[1] + v2[2]*v2[2]).sqrt();
                    if (len2 - 1.0).abs() < 0.02 {
                        G_DUMPER.add_offset("Attachment", "SecondaryAxis", off2);
                        eprintln!("  Attachment::SecondaryAxis at +0x{:x}", off2);
                        break;
                    }
                }
                break;
            }
        }
    }

    for off in (0..0x100).step_by(1) {
        let v = match memory::read::<u8>(mem, a + off) {
            Some(v) => v,
            None => continue,
        };
        if v == 1 {
            if let Some(pos) = G_DUMPER.get_offset("Attachment", "Position") {
                if off >= pos && off <= pos + 3 { continue; }
            }
            G_DUMPER.add_offset("Attachment", "Visible", off);
            eprintln!("  Attachment::Visible at +0x{:x}", off);
            break;
        }
    }

    if atts.len() >= 2 {
        let a2 = atts[1];
        let pos = G_DUMPER.get_offset("Attachment", "Position").unwrap_or(usize::MAX);
        if pos != usize::MAX {
            if let Some(p1) = memory::read::<[f32; 3]>(mem, a + pos) {
                if let Some(p2) = memory::read::<[f32; 3]>(mem, a2 + pos) {
                    let diff = (p1[0] - p2[0]).abs() + (p1[1] - p2[1]).abs() + (p1[2] - p2[2]).abs();
                    if diff > 0.01 {
                        for wp_off in (0..0x100).step_by(4) {
                            if wp_off == pos || wp_off == pos + 4 || wp_off == pos + 8 { continue; }
                            let wpa = match memory::read::<[f32; 3]>(mem, a + wp_off) {
                                Some(wpa) => wpa,
                                None => continue,
                            };
                            if (wpa[0] - p1[0]).abs() < 0.01 && (wpa[1] - p1[1]).abs() < 0.01 && (wpa[2] - p1[2]).abs() < 0.01 {
                                continue;
                            }
                            let wpa2 = match memory::read::<[f32; 3]>(mem, a2 + wp_off) {
                                Some(wpa2) => wpa2,
                                None => continue,
                            };
                            if (wpa2[0] - p2[0]).abs() > 0.01 || (wpa2[1] - p2[1]).abs() > 0.01 || (wpa2[2] - p2[2]).abs() > 0.01 {
                                G_DUMPER.add_offset("Attachment", "WorldPosition", wp_off);
                                eprintln!("  Attachment::WorldPosition at +0x{:x}", wp_off);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    true
}
