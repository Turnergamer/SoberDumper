use std::fs::File;
use crate::memory;
use crate::rtti;
use crate::dumper::{G_DUMPER, G_DATA_MODEL_ADDR};

pub fn dump(mem: &File) -> bool {
    eprintln!("[datamodel_ext]");

    let dm_addr = unsafe { G_DATA_MODEL_ADDR };
    if dm_addr < 0x10000 { return true; }

    // UniverseId: int64, typically > 0, not PlaceId/CreatorId/GameId
    let skip = [
        G_DUMPER.get_offset("DataModel", "PlaceId").unwrap_or(0),
        G_DUMPER.get_offset("DataModel", "CreatorId").unwrap_or(0),
        G_DUMPER.get_offset("DataModel", "GameId").unwrap_or(0),
        G_DUMPER.get_offset("DataModel", "JobId").unwrap_or(0),
        G_DUMPER.get_offset("DataModel", "Workspace").unwrap_or(0),
    ];

    for off in (0..0x400).step_by(8) {
        if skip.contains(&off) { continue; }
        let v = match memory::read::<i64>(mem, dm_addr + off) {
            Some(v) => v,
            None => continue,
        };
        if v > 0 && v < 100_000_000_000 {
            let prev = memory::read::<i64>(mem, dm_addr + off.wrapping_sub(8)).unwrap_or(-1);
            let next = memory::read::<i64>(mem, dm_addr + off + 8).unwrap_or(-1);
            if (prev < 0 || prev > 100_000_000_000) && (next < 0 || next > 100_000_000_000) {
                G_DUMPER.add_offset("DataModel", "UniverseId", off);
                eprintln!("  DataModel::UniverseId at +0x{:x} ({})", off, v);
                break;
            }
        }
    }

    // PrivateServerId: string pointer (UUID format)
    for off in (0..0x800).step_by(8) {
        if skip.contains(&off) { continue; }
        let ptr = match memory::read::<usize>(mem, dm_addr + off) {
            Some(p) => p,
            None => continue,
        };
        if ptr < 0x10000 { continue; }
        if let Some(s) = memory::read_string(mem, ptr, 48) {
            let bytes = s.as_bytes();
            if bytes.len() == 36
                && bytes[8] == b'-' && bytes[13] == b'-'
                && bytes[18] == b'-' && bytes[23] == b'-'
            {
                G_DUMPER.add_offset("DataModel", "PrivateServerId", off);
                eprintln!("  DataModel::PrivateServerId at +0x{:x}", off);
                break;
            }
        }
    }

    // PrivateServerOwnerId: int64 (a Roblox UserId, typically < 10^8)
    for off in (0..0x800).step_by(8) {
        if skip.contains(&off) { continue; }
        let v = match memory::read_i64_sane(mem, dm_addr + off) {
            Some(v) => v,
            None => continue,
        };
        if v > 100 && v < 1_000_000_000 {
            let uid = G_DUMPER.get_offset("DataModel", "UniverseId").unwrap_or(usize::MAX);
            let pid = G_DUMPER.get_offset("DataModel", "PrivateServerId").unwrap_or(usize::MAX);
            if off == uid || off == pid { continue; }
            G_DUMPER.add_offset("DataModel", "PrivateServerOwnerId", off);
            eprintln!("  DataModel::PrivateServerOwnerId at +0x{:x} ({})", off, v);
            break;
        }
    }

    // SavaVersion: u32 (small integer, usually 0-50)
    for off in (0..0x200).step_by(4) {
        let v = match memory::read::<u32>(mem, dm_addr + off) {
            Some(v) => v,
            None => continue,
        };
        if v > 0 && v < 100 {
            let prev = memory::read::<u32>(mem, dm_addr + off.wrapping_sub(4)).unwrap_or(u32::MAX);
            let next = memory::read::<u32>(mem, dm_addr + off + 4).unwrap_or(u32::MAX);
            if prev > 100 && next > 100 {
                G_DUMPER.add_offset("DataModel", "SavaVersion", off);
                eprintln!("  DataModel::SavaVersion at +0x{:x}", off);
                break;
            }
        }
    }

    // Find more services via RTTI in DataModel
    for &(name, ns) in &[
        ("RunService@RBX", "RunService"),
        ("UserInputService@RBX", "UserInputService"),
        ("HttpService@RBX", "HttpService"),
        ("MarketplaceService@RBX", "MarketplaceService"),
        ("TeleportService@RBX", "TeleportService"),
        ("SocialService@RBX", "SocialService"),
        ("Chat@RBX", "Chat"),
        ("BadgeService@RBX", "BadgeService"),
        ("InsertService@RBX", "InsertService"),
        ("ScriptContext@RBX", "ScriptContext"),
        ("ContentProvider@RBX", "ContentProvider"),
        ("CorePackages@RBX", "CorePackages"),
    ] {
        if let Some(off) = rtti::find(mem, dm_addr, name, 0x2000, 8) {
            G_DUMPER.add_offset(ns, "Service", off);
            eprintln!("  {} at +0x{:x}", ns, off);

            // For specific services with known patterns
            if ns == "RunService" {
                for so_off in (0..0x100).step_by(4) {
                    if let Some(v) = memory::read_f32(mem, dm_addr + off + so_off) {
                        if v > 0.0 && v < 1.0 {
                            G_DUMPER.add_offset("RunService", "SimulationRate", so_off);
                            break;
                        }
                    }
                }
            }
        }
    }

    true
}
