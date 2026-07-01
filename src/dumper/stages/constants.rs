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

/// Find MouseService via DataModel children, override baseline.
fn dump_mouse_service(mem: &File) {
    let dm_addr = unsafe { crate::dumper::G_DATA_MODEL_ADDR };
    let cs = G_DUMPER.get_offset("Instance", "ChildrenStart").unwrap_or(0);
    let ce = G_DUMPER.get_offset("Instance", "ChildrenEnd").unwrap_or(0);

    let mouse_svc = if cs > 0 && ce > 0 {
        let kids = collect_children(mem, dm_addr, cs, ce);
        kids.iter().find_map(|&c| {
            if let Some(r) = rtti::scan_rtti(mem, c) {
                if r.name == "MouseService@RBX" { Some(c) } else { None }
            } else { None }
        })
    } else {
        None
    };

    if let Some(ms) = mouse_svc {
        eprintln!("  MouseService @ 0x{:x}", ms);
        if let Some(io) = rtti::find(mem, ms, "InputObject@RBX", 0x200, 8) {
            G_DUMPER.add_offset("MouseService", "InputObject", io);
            eprintln!("  >> MouseService::InputObject = 0x{:x} (dynamic)", io);
        }
    }
}

/// Find Stats/StatsItem, override baseline values.
fn dump_stats(mem: &File) {
    let dm_addr = unsafe { crate::dumper::G_DATA_MODEL_ADDR };
    let cs = G_DUMPER.get_offset("Instance", "ChildrenStart").unwrap_or(0);
    let ce = G_DUMPER.get_offset("Instance", "ChildrenEnd").unwrap_or(0);

    let stats_svc = if cs > 0 && ce > 0 {
        let kids = collect_children(mem, dm_addr, cs, ce);
        kids.iter().find_map(|&c| {
            if let Some(r) = rtti::scan_rtti(mem, c) {
                if r.name == "Stats@RBX" || r.name == "StatsItem@RBX" { Some(c) }
                else {
                    let grandkids = collect_children(mem, c, cs, ce);
                    grandkids.iter().find_map(|&gk| {
                        if let Some(r2) = rtti::scan_rtti(mem, gk) {
                            if r2.name == "StatsItem@RBX" { Some(gk) } else { None }
                        } else { None }
                    })
                }
            } else { None }
        })
    } else {
        None
    };

    if let Some(stats) = stats_svc {
        eprintln!("  Stats @ 0x{:x}", stats);

        for off in (0..0x300).step_by(4) {
            let v = match memory::read_f32(mem, stats + off) {
                Some(v) => v,
                None => continue,
            };
            if v > 0.0 && v < 1_000_000.0 {
                G_DUMPER.add_offset("StatsItem", "Value", off);
                eprintln!("  >> StatsItem::Value = 0x{:x} (dynamic)", off);
                break;
            }
        }

        for off in (0..0x500).step_by(8) {
            let ptr = match memory::read::<usize>(mem, stats + off) {
                Some(p) => p,
                None => continue,
            };
            if ptr < 0x10000 { continue; }
            if let Some(s) = memory::read_name_fmt(mem, ptr) {
                if !s.is_empty() && s.len() < 64 && G_DUMPER.get_offset("StatsItem", "Name").is_none() {
                    G_DUMPER.add_offset("StatsItem", "Name", off);
                    eprintln!("  >> StatsItem::Name = 0x{:x} (dynamic)", off);
                } else if G_DUMPER.get_offset("StatsItem", "Name").is_some()
                    && G_DUMPER.get_offset("StatsItem", "DisplayName").is_none()
                    && !s.is_empty() && s.len() < 64
                {
                    G_DUMPER.add_offset("StatsItem", "DisplayName", off);
                    eprintln!("  >> StatsItem::DisplayName = 0x{:x} (dynamic)", off);
                    break;
                }
            }
        }

        let val_off = G_DUMPER.get_offset("StatsItem", "Value").unwrap_or(0);
        if val_off > 0 {
            for off in (val_off + 4..val_off + 0x50).step_by(4) {
                let v = match memory::read_f32(mem, stats + off) {
                    Some(v) => v,
                    None => continue,
                };
                if v >= 0.0 && v < 1_000_000.0 {
                    if G_DUMPER.get_offset("StatsItem", "AvgValue").is_none() {
                        G_DUMPER.add_offset("StatsItem", "AvgValue", off);
                        eprintln!("  >> StatsItem::AvgValue = 0x{:x} (dynamic)", off);
                    } else if G_DUMPER.get_offset("StatsItem", "AvgValuePrev").is_none() {
                        G_DUMPER.add_offset("StatsItem", "AvgValuePrev", off);
                        eprintln!("  >> StatsItem::AvgValuePrev = 0x{:x} (dynamic)", off);
                        break;
                    }
                }
            }
        }
    }
}

pub fn dump(mem: &File) -> bool {
    eprintln!("[mouse/stats dynamic]");
    dump_mouse_service(mem);
    dump_stats(mem);
    true
}
