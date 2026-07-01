use std::fs::File;
use crate::memory;
use crate::rtti;
use crate::dumper::{G_DUMPER, G_DATA_MODEL_ADDR, G_WORKSPACE_ADDR};

fn find_parent_offset(mem: &File, addr: usize) -> Option<usize> {
    rtti::find(mem, addr, "DataModel@RBX", 0x400, 8)
}

fn try_children_verified(mem: &File, addr: usize, parent_off: usize) -> Option<(usize, usize)> {
    for start_off in (0..0x300).step_by(8) {
        if start_off == parent_off { continue; }
        let start_ptr = match memory::read::<usize>(mem, addr + start_off) {
            Some(p) => p,
            None => continue,
        };
        if start_ptr < 0x10000 { continue; }

        for end_off in (0..0x20).step_by(8) {
            let end_ptr = match memory::read::<usize>(mem, start_ptr + end_off) {
                Some(p) => p,
                None => continue,
            };
            if end_ptr < 0x10000 { continue; }

            let node = match memory::read::<usize>(mem, start_ptr) {
                Some(n) => n,
                None => continue,
            };
            if node < 0x10000 { continue; }

            let mut valid = 0u32;
            let mut n = node;
            let mut failed = false;
            for _ in 0..500 {
                if n == end_ptr { break; }
                let child = match memory::read::<usize>(mem, n) {
                    Some(c) => c,
                    None => { failed = true; break; }
                };
                if child < 0x10000 { failed = true; break; }
                let vtable = match memory::read::<usize>(mem, child) {
                    Some(v) => v,
                    None => { failed = true; break; }
                };
                if vtable < 0x10000 { failed = true; break; }
                let parent = match memory::read::<usize>(mem, child + parent_off) {
                    Some(p) => p,
                    None => { failed = true; break; }
                };
                if parent != addr { failed = true; break; }
                valid += 1;
                n += 0x10;
            }
            if !failed && valid >= 2 {
                return Some((start_off, end_off));
            }
        }
    }
    None
}

fn try_children_no_verify(mem: &File, addr: usize) -> Option<(usize, usize)> {
    for start_off in (0..0x300).step_by(8) {
        let start_ptr = match memory::read::<usize>(mem, addr + start_off) {
            Some(p) => p,
            None => continue,
        };
        if start_ptr < 0x10000 { continue; }

        for end_off in (0..0x20).step_by(8) {
            let end_ptr = match memory::read::<usize>(mem, start_ptr + end_off) {
                Some(p) => p,
                None => continue,
            };
            if end_ptr < 0x10000 { continue; }

            let node = match memory::read::<usize>(mem, start_ptr) {
                Some(n) => n,
                None => continue,
            };
            if node < 0x10000 { continue; }

            let mut valid = 0u32;
            let mut n = node;
            let mut failed = false;
            for _ in 0..500 {
                if n == end_ptr { break; }
                let child = match memory::read::<usize>(mem, n) {
                    Some(c) => c,
                    None => { failed = true; break; }
                };
                if child < 0x10000 { failed = true; break; }
                let vtable = match memory::read::<usize>(mem, child) {
                    Some(v) => v,
                    None => { failed = true; break; }
                };
                if vtable < 0x10000 { failed = true; break; }
                if rtti::scan_rtti(mem, child).is_some() {
                    valid += 1;
                } else {
                    failed = true;
                    break;
                }
                n += 0x10;
            }
            if !failed && valid >= 2 {
                return Some((start_off, end_off));
            }
        }
    }
    None
}

fn try_children_bruteforce(mem: &File, addr: usize, parent_off: usize) -> Option<(usize, usize)> {
    for start_off in (0..0x300).step_by(8) {
        if start_off == parent_off { continue; }
        let head_ptr = match memory::read::<usize>(mem, addr + start_off) {
            Some(p) => p,
            None => continue,
        };
        if head_ptr < 0x10000 { continue; }

        for stride in &[0x10usize, 0x18, 0x20, 0x08] {
            for end_off in (0..0x20).step_by(8) {
                let sentinel = match memory::read::<usize>(mem, head_ptr + end_off) {
                    Some(s) => s,
                    None => continue,
                };
                let first = match memory::read::<usize>(mem, head_ptr) {
                    Some(f) => f,
                    None => continue,
                };
                if first < 0x10000 { continue; }

                let mut valid = 0u32;
                let mut n = first;
                let mut failed = false;
                for _ in 0..500 {
                    if n == sentinel || n == 0 || n == head_ptr { break; }
                    let child = match memory::read::<usize>(mem, n) {
                        Some(c) => c,
                        None => { failed = true; break; }
                    };
                    if child < 0x10000 { failed = true; break; }
                    let vtable = match memory::read::<usize>(mem, child) {
                        Some(v) => v,
                        None => { failed = true; break; }
                    };
                    if vtable < 0x10000 { failed = true; break; }
                    let parent = match memory::read::<usize>(mem, child + parent_off) {
                        Some(p) => p,
                        None => { failed = true; break; }
                    };
                    if parent != addr { failed = true; break; }
                    valid += 1;
                    n += stride;
                }
                if !failed && valid >= 1 {
                    return Some((start_off, end_off));
                }
            }
        }
    }
    None
}

/// Scan for the ClassDescriptor pointer in an Instance.
/// The ClassDescriptor is a stable RTTI-like pointer unique per class.
fn find_class_descriptor(mem: &File, addr: usize) -> Option<usize> {
    for off in (0..0x80).step_by(8) {
        let ptr = memory::read::<usize>(mem, addr + off)?;
        if ptr < 0x10000 { continue; }
        let vtable = memory::read::<usize>(mem, ptr)?;
        if !(0x10000..0x7fffffffffff).contains(&vtable) { continue; }
        G_DUMPER.add_offset("Instance", "ClassDescriptor", off);
        return Some(off);
    }
    None
}

/// Find ClassName pointer: scan for a Roblox name string (SSO or pointer)
/// matching the class name obtained from RTTI.
fn find_class_name(mem: &File, addr: usize) -> Option<usize> {
    if let Some(rtti) = rtti::scan_rtti(mem, addr) {
        let class_name = rtti.name.split('@').next().unwrap_or(&rtti.name);
        for off in (0..0x80).step_by(8) {
            let ptr = memory::read::<usize>(mem, addr + off)?;
            if ptr >= 0x10000 {
                if let Some(s) = memory::read_name_fmt(mem, ptr) {
                    if s == class_name {
                        G_DUMPER.add_offset("Instance", "ClassName", off);
                        return Some(off);
                    }
                }
            }
            if let Some(s) = read_sso(mem, addr + off) {
                if s == class_name {
                    G_DUMPER.add_offset("Instance", "ClassName", off);
                    return Some(off);
                }
            }
        }
    }
    None
}

pub fn dump(mem: &File) -> bool {
    eprintln!("[instance]");

    let dm_addr = unsafe { G_DATA_MODEL_ADDR };
    let ws_off = G_DUMPER.get_offset("DataModel", "Workspace")
        .expect("No Workspace offset");
    let ws_addr = memory::read::<usize>(mem, dm_addr + ws_off)
        .expect("Failed to read Workspace");
    eprintln!("  Workspace @ 0x{:x}", ws_addr);
    unsafe { G_WORKSPACE_ADDR = ws_addr; }

    // Name: scan for "Workspace" string
    for off in (0..0x400).step_by(8) {
        let ptr = memory::read::<usize>(mem, ws_addr + off);
        if let Some(p) = ptr {
            if p >= 0x10000 {
                if let Some(s) = memory::read_name_fmt(mem, p) {
                    if s == "Workspace" { G_DUMPER.add_offset("Instance", "Name", off); break; }
                }
            }
        }
        if let Some(s) = read_sso(mem, ws_addr + off) {
            if s == "Workspace" { G_DUMPER.add_offset("Instance", "Name", off); break; }
        }
    }

    // Find Parent offset internally
    let parent_off = find_parent_offset(mem, ws_addr)
        .unwrap_or(0x70);

    // Children via linked-list walk (3 strategies)
    let children = try_children_verified(mem, ws_addr, parent_off)
        .or_else(|| try_children_no_verify(mem, ws_addr))
        .or_else(|| try_children_bruteforce(mem, ws_addr, parent_off));

    if let Some((cs, ce)) = children {
        G_DUMPER.add_offset("Instance", "ChildrenStart", cs);
        G_DUMPER.add_offset("Instance", "ChildrenEnd", ce);
    } else {
        eprintln!("  ChildrenStart/End not found via linked-list walk");
    }

    // ClassDescriptor
    find_class_descriptor(mem, ws_addr);

    // ClassName
    find_class_name(mem, ws_addr);

    // Parent offset (for DataModel parent = null)
    // Already found above via RTTI, register it
    if parent_off != 0x70 {
        G_DUMPER.add_offset("Instance", "Parent", parent_off);
    }

    true
}

fn read_sso(mem: &File, addr: usize) -> Option<String> {
    let size_byte = memory::read::<u8>(mem, addr)?;
    let len = size_byte as usize;
    if len <= 15 {
        let buf = memory::read_bytes(mem, addr + 1, 15)?;
        let end = buf.iter().position(|&b| b == 0).unwrap_or(len);
        let s = String::from_utf8_lossy(&buf[..end]).to_string();
        if s.len() == len { Some(s) } else { None }
    } else {
        let ptr = memory::read::<usize>(mem, addr + 8)?;
        let len2 = memory::read::<usize>(mem, addr + 16)?;
        if ptr < 0x10000 || len2 > 256 { return None; }
        memory::read_string(mem, ptr, len2)
    }
}
