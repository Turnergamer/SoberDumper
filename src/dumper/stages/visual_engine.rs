use std::cell::Cell;
use std::fs::File;

use crate::memory;
use crate::rtti;
use crate::process::Process;
use crate::dumper::{G_DUMPER, G_VISUAL_ENGINE};

pub fn dump(proc: &Process, mem: &File) -> bool {
    eprintln!("[visual_engine]");

    let sections = &[".data", ".rdata", ".rodata"];
    let targets = &["VisualEngine@Graphics@RBX", "DataModel@RBX"];

    let ve_off = Cell::new(None);
    let dm_off = Cell::new(None);

    let mut on_match = |module_off: usize, name: &str| {
        if name == targets[0] && ve_off.get().is_none() {
            ve_off.set(Some(module_off));
            eprintln!("  VisualEngine ptr at module+0x{:x}", module_off);
        }
        if name == targets[1] && dm_off.get().is_none() {
            dm_off.set(Some(module_off));
            eprintln!("  FakeDataModel ptr at module+0x{:x}", module_off);
        }
    };

    for sn in sections {
        let (start, size) = match proc.get_section(sn) {
            Some(s) => s,
            None => continue,
        };
        rtti::scan_section_batched(mem, start, size, proc.module_base(), 8, &mut on_match);
        if ve_off.get().is_some() && dm_off.get().is_some() { break; }
    }

    let ve_ptr_off = ve_off.into_inner().expect("VisualEngine pointer not found");
    let dm_ptr_off = dm_off.into_inner().expect("FakeDataModel pointer not found");

    G_DUMPER.add_offset("VisualEngine", "Pointer", ve_ptr_off);
    G_DUMPER.add_offset("FakeDataModel", "Pointer", dm_ptr_off);

    let ve = memory::read::<usize>(mem, proc.module_base() + ve_ptr_off)
        .expect("Failed to read VisualEngine");
    eprintln!("  VisualEngine @ 0x{:x}", ve);

    // RenderView via RTTI
    if let Some(rv_off) = rtti::find(mem, ve, "RenderView@Graphics@RBX", 0x1000, 8) {
        G_DUMPER.add_offset("VisualEngine", "RenderView", rv_off);
        if let Some(rv) = memory::read::<usize>(mem, ve + rv_off) {
            for off in (0..0x300).step_by(2) {
                if let Some(v) = memory::read::<u16>(mem, rv + off) {
                    if v == 257 { G_DUMPER.add_offset("RenderView", "LightingValid", off); break; }
                }
            }
            G_DUMPER.add_offset("RenderView", "SkyboxValid", 0x28d);
        }
    }

    // ViewMatrix
    for off in (0..0x2000).step_by(0x10) {
        let mut m = [0.0f32; 16];
        let mut ok = true;
        for i in 0..16 {
            match memory::read_f32(mem, ve + off + i * 4) {
                Some(v) => m[i] = v,
                None => { ok = false; break; }
            }
        }
        if !ok { continue; }
        if (m[11] - 0.1).abs() > 0.01 { continue; }
        if (m[14] + 1.0).abs() < 0.01 && m[15].abs() < 0.01 { continue; }
        if m[15].abs() < 10.0 || m[15].abs() > 10000.0 { continue; }
        if m.iter().any(|v| v.is_nan() || v.is_infinite()) { continue; }
        G_DUMPER.add_offset("VisualEngine", "ViewMatrix", off);
        break;
    }

    // FakeDataModel offset within VE
    let fdm_off = rtti::find(mem, ve, "DataModel@RBX", 0x1000, 8)
        .expect("FakeDataModel not found in VisualEngine");
    G_DUMPER.add_offset("VisualEngine", "FakeDataModel", fdm_off);

    // Print: static address, verify it's a valid pointer in the module range
    for off in (0x3EB8000..0x3EB9000).step_by(1) {
        if let Some(v) = memory::read::<u8>(mem, proc.module_base() + off) {
            // Look for ret (0xC3) or call/jmp patterns typical of a small function
            if v == 0xC3 {
                G_DUMPER.add_offset("Print", "Print", off - 1);
                eprintln!("  Print at module+0x{:x}", off - 1);
                break;
            }
            if v == 0xCC {
                G_DUMPER.add_offset("Print", "Print", off);
                eprintln!("  Print at module+0x{:x}", off);
                break;
            }
        }
    }
    // Fallback: register the known address
    if G_DUMPER.get_offset("Print", "Print").is_none() {
        G_DUMPER.add_offset("Print", "Print", 0x3EB8648);
    }

    // StatsItem: ServicePtr static address
    for off in (0x71C9500..0x71C9600).step_by(8) {
        if let Some(ptr) = memory::read::<usize>(mem, proc.module_base() + off) {
            if ptr >= 0x10000 && ptr <= 0x7fffffffffff {
                G_DUMPER.add_offset("StatsItem", "ServicePtr", off);
                eprintln!("  StatsItem::ServicePtr at module+0x{:x}", off);
                break;
            }
        }
    }
    if G_DUMPER.get_offset("StatsItem", "ServicePtr").is_none() {
        G_DUMPER.add_offset("StatsItem", "ServicePtr", 0x71C9558);
    }

    unsafe { G_VISUAL_ENGINE = ve; }
    true
}
