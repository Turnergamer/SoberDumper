use std::fs::File;
use crate::memory;
use crate::rtti;
use crate::dumper::{G_DUMPER, G_VISUAL_ENGINE, G_DATA_MODEL_ADDR};

fn find_int64(mem: &File, addr: usize, max_off: usize, skip: &[usize]) -> Option<usize> {
    for off in (0..max_off).step_by(8) {
        if skip.contains(&off) || off < 8 { continue; }
        let v = memory::read::<i64>(mem, addr + off)?;
        if v > 0 && v < 100_000_000_000 {
            let next = memory::read::<i64>(mem, addr + off + 8).unwrap_or(-1);
            let prev = memory::read::<i64>(mem, addr + off - 8).unwrap_or(-1);
            if next < 0 || next > 100_000_000_000 || prev < 0 || prev > 100_000_000_000 {
                continue;
            }
            return Some(off);
        }
    }
    None
}

pub fn dump(mem: &File) -> bool {
    eprintln!("[data_model]");

    let ve = unsafe { G_VISUAL_ENGINE };
    let fdm_off = G_DUMPER.get_offset("VisualEngine", "FakeDataModel")
        .expect("No FakeDataModel offset");

    let fdm = memory::read::<usize>(mem, ve + fdm_off)
        .expect("Failed to read FakeDataModel ptr");

    let dm_off = rtti::find(mem, fdm, "DataModel@RBX", 0x1000, 8)
        .expect("RealDataModel offset not found");
    G_DUMPER.add_offset("FakeDataModel", "RealDataModel", dm_off);
    let dm_addr = memory::read::<usize>(mem, fdm + dm_off)
        .expect("Failed to read RealDataModel addr");
    unsafe { G_DATA_MODEL_ADDR = dm_addr; }
    eprintln!("  DataModel @ 0x{:x}", dm_addr);

    // Workspace via RTTI
    if let Some(ws) = rtti::find(mem, dm_addr, "Workspace@RBX", 0x1000, 8) {
        G_DUMPER.add_offset("DataModel", "Workspace", ws);
    }

    // PlaceId: int64 > 0
    if let Some(pi) = find_int64(mem, dm_addr, 0x300, &[]) {
        G_DUMPER.add_offset("DataModel", "PlaceId", pi);
        eprintln!("  PlaceId at +0x{:x}", pi);
    }

    // CreatorId / GameId: scan for int64s past PlaceId range
    let mut skip = vec![];
    if let Some(pi) = G_DUMPER.get_offset("DataModel", "PlaceId") { skip.push(pi); }
    if let Some(ws) = G_DUMPER.get_offset("DataModel", "Workspace") { skip.push(ws); }
    if let Some(id) = find_int64(mem, dm_addr, 0x300, &skip) {
        G_DUMPER.add_offset("DataModel", "CreatorId", id);
        eprintln!("  CreatorId at +0x{:x}", id);
        skip.push(id);
    }
    if let Some(id) = find_int64(mem, dm_addr, 0x300, &skip) {
        G_DUMPER.add_offset("DataModel", "GameId", id);
        eprintln!("  GameId at +0x{:x}", id);
        skip.push(id);
    }

    // JobId: scan for UUID-format string pointer
    for off in (0..0x200).step_by(8) {
        if skip.contains(&off) { continue; }
        let ptr = match memory::read::<usize>(mem, dm_addr + off) {
            Some(p) => p,
            None => continue,
        };
        if ptr < 0x10000 { continue; }
        if let Some(s) = memory::read_string(mem, ptr, 64) {
            let bytes = s.as_bytes();
            if bytes.len() == 36
                && bytes[8] == b'-' && bytes[13] == b'-'
                && bytes[18] == b'-' && bytes[23] == b'-'
            {
                G_DUMPER.add_offset("DataModel", "JobId", off);
                eprintln!("  JobId at +0x{:x}", off);
                skip.push(off);
                break;
            }
        }
    }

    // ServerIp: scan for "IP:Port" string pointer
    for off in (0..0x700).step_by(8) {
        if skip.contains(&off) { continue; }
        let ptr = match memory::read::<usize>(mem, dm_addr + off) {
            Some(p) => p,
            None => continue,
        };
        if ptr < 0x10000 { continue; }
        if let Some(s) = memory::read_string(mem, ptr, 64) {
            if s.contains(':') && s.chars().filter(|&c| c == '.').count() >= 3 {
                G_DUMPER.add_offset("DataModel", "ServerIp", off);
                eprintln!("  ServerIp at +0x{:x}", off);
                break;
            }
        }
    }

    true
}
