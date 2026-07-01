use std::fs::File;
use crate::memory;
use crate::rtti;
use crate::dumper::{G_DUMPER, G_DATA_MODEL_ADDR, G_WORKSPACE_ADDR};

pub fn dump(mem: &File) -> bool {
    eprintln!("[workspace]");

    let ws_off = G_DUMPER.get_offset("DataModel", "Workspace")
        .expect("No Workspace offset in DataModel");

    let ws_addr = memory::read::<usize>(mem, unsafe { G_DATA_MODEL_ADDR } + ws_off)
        .expect("Failed to read Workspace addr");
    eprintln!("  Workspace @ 0x{:x}", ws_addr);
    unsafe { G_WORKSPACE_ADDR = ws_addr; }

    // CurrentCamera via RTTI
    if let Some(cc) = rtti::find(mem, ws_addr, "Camera@RBX", 0x1000, 8) {
        G_DUMPER.add_offset("Workspace", "CurrentCamera", cc);
    }

    // World via RTTI
    if let Some(world_off) = rtti::find(mem, ws_addr, "World@RBX", 0x1000, 8) {
        G_DUMPER.add_offset("Workspace", "World", world_off);
        eprintln!("  World at +0x{:x}", world_off);

        if let Some(world_addr) = memory::read::<usize>(mem, ws_addr + world_off) {
            if world_addr >= 0x10000 {
                // Gravity: vec3 (default (0, -196.2, 0))
                for off in (0..0x300).step_by(4) {
                    let v = match memory::read::<[f32; 3]>(mem, world_addr + off) {
                        Some(v) => v,
                        None => continue,
                    };
                    if v.iter().any(|x| x.is_nan() || x.is_infinite() || x.is_subnormal()) { continue; }
                    if (v[0]).abs() < 0.01 && (v[2]).abs() < 0.01 && v[1] < 0.0 && v[1] > -2000.0 {
                        G_DUMPER.add_offset("World", "Gravity", off);
                        eprintln!("  World::Gravity at +0x{:x}", off);
                        break;
                    }
                }

                // Primitives: find pointer to array of Primitive pointers
                for off in (0..0x800).step_by(8) {
                    let ptr = match memory::read::<usize>(mem, world_addr + off) {
                        Some(p) => p,
                        None => continue,
                    };
                    if ptr < 0x10000 { continue; }

                    let first_slot = match memory::read::<usize>(mem, ptr) {
                        Some(s) => s,
                        None => continue,
                    };
                    if first_slot >= 0x10000 {
                        if let Some(r) = rtti::scan_rtti(mem, first_slot) {
                            if r.name == "Primitive@RBX" {
                                G_DUMPER.add_offset("World", "Primitives", off);
                                eprintln!("  World::Primitives at +0x{:x}", off);
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
