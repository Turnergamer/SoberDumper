use std::fs::File;
use crate::memory;
use crate::dumper::{G_DUMPER, G_WORKSPACE_ADDR, find_instances};

pub fn dump(mem: &File) -> bool {
    eprintln!("[sound]");

    let ws_addr = unsafe { G_WORKSPACE_ADDR };

    let mut sounds = find_instances(mem, ws_addr, "Sound@RBX", 3);
    if sounds.is_empty() {
        for off in (0..0x2000).step_by(8) {
            if let Some(ptr) = memory::read::<usize>(mem, ws_addr + off) {
                if ptr >= 0x10000 {
                    if let Some(r) = crate::rtti::scan_rtti(mem, ptr) {
                        if r.name == "Sound@RBX" { sounds.push(ptr); break; }
                    }
                }
            }
        }
    }

    if sounds.is_empty() { eprintln!("  No Sound found"); return true; }
    eprintln!("  Found {} Sound(s)", sounds.len());
    let s = sounds[0];

    for off in (0..0x200).step_by(4) {
        let v = match memory::read_f32(mem, s + off) {
            Some(v) => v,
            None => continue,
        };
        if (v - 1.0).abs() < 0.05 && v >= 0.0 && v <= 10.0 {
            G_DUMPER.add_offset("Sound", "Volume", off);
            eprintln!("  Sound::Volume at +0x{:x} ({})", off, v);
            break;
        }
    }

    for off in (0..0x200).step_by(4) {
        let v = match memory::read_f32(mem, s + off) {
            Some(v) => v,
            None => continue,
        };
        if (v - 1.0).abs() < 0.05 && v >= 0.0 && v <= 10.0 {
            let vol_off = G_DUMPER.get_offset("Sound", "Volume").unwrap_or(usize::MAX);
            if off == vol_off { continue; }
            G_DUMPER.add_offset("Sound", "Pitch", off);
            eprintln!("  Sound::Pitch at +0x{:x} ({})", off, v);
            break;
        }
    }

    for off in (0..0x300).step_by(8) {
        let ptr = match memory::read::<usize>(mem, s + off) {
            Some(p) => p,
            None => continue,
        };
        if ptr < 0x10000 { continue; }
        if let Some(sid) = memory::read_name_fmt(mem, ptr) {
            if sid.starts_with("rbxasset://") || sid.starts_with("http") {
                G_DUMPER.add_offset("Sound", "SoundId", off);
                eprintln!("  Sound::SoundId at +0x{:x}", off);
                break;
            }
        }
    }

    for off in (0..0x100).step_by(1) {
        let v = match memory::read::<u8>(mem, s + off) {
            Some(v) => v,
            None => continue,
        };
        if v != 0 && v != 1 { continue; }
        let next = memory::read::<u8>(mem, s + off + 1).unwrap_or(2);
        if (v == 0 && next == 1) || (v == 1 && next <= 1) {
            G_DUMPER.add_offset("Sound", "Looped", off);
            eprintln!("  Sound::Looped at +0x{:x}", off);
            break;
        }
    }

    {
        let skip_vol = G_DUMPER.get_offset("Sound", "Volume");
        let skip_pit = G_DUMPER.get_offset("Sound", "Pitch");
        for off in (0..0x200).step_by(4) {
            if skip_vol == Some(off) || skip_pit == Some(off) { continue; }
            let v = match memory::read_f32(mem, s + off) {
                Some(v) => v,
                None => continue,
            };
            if (v - 1.0).abs() < 0.05 && v >= 0.0 && v <= 10.0 {
                G_DUMPER.add_offset("Sound", "PlaybackSpeed", off);
                eprintln!("  Sound::PlaybackSpeed at +0x{:x} ({})", off, v);
                break;
            }
        }
    }

    for off in (0..0x200).step_by(4) {
        let v = match memory::read_f32(mem, s + off) {
            Some(v) => v,
            None => continue,
        };
        if v.abs() > 0.01 { continue; }
        let vol = G_DUMPER.get_offset("Sound", "Volume").unwrap_or(usize::MAX);
        let pit = G_DUMPER.get_offset("Sound", "Pitch").unwrap_or(usize::MAX);
        let spd = G_DUMPER.get_offset("Sound", "PlaybackSpeed").unwrap_or(usize::MAX);
        if off == vol || off == pit || off == spd { continue; }
        G_DUMPER.add_offset("Sound", "TimePosition", off);
        eprintln!("  Sound::TimePosition at +0x{:x}", off);
        break;
    }

    let looped_off = G_DUMPER.get_offset("Sound", "Looped").unwrap_or(usize::MAX);
    for off in (0..0x100).step_by(1) {
        let v = match memory::read::<u8>(mem, s + off) {
            Some(v) => v,
            None => continue,
        };
        if v != 0 && v != 1 { continue; }
        if (off as isize - looped_off as isize).abs() <= 1 { continue; }
        let next = match memory::read::<u8>(mem, s + off + 1) {
            Some(n) => n,
            None => continue,
        };
        if next == 0 || next == 1 {
            G_DUMPER.add_offset("Sound", "PlayOnRemove", off);
            eprintln!("  Sound::PlayOnRemove at +0x{:x}", off);
            break;
        }
    }

    true
}
