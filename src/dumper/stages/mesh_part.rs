use std::fs::File;
use crate::memory;
use crate::rtti;
use crate::dumper::{G_DUMPER, G_WORKSPACE_ADDR};

fn collect_children(mem: &File, addr: usize, cs: usize, ce: usize) -> Vec<usize> {
    let mut out = vec![];
    let head = match memory::read::<usize>(mem, addr + cs) {
        Some(h) => h, None => return out,
    };
    let first = match memory::read::<usize>(mem, head) {
        Some(f) => f, None => return out,
    };
    let last = match memory::read::<usize>(mem, head + ce) {
        Some(l) => l, None => return out,
    };
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

/// Find MeshPart instances by scanning Workspace and its children.
fn find_mesh_parts(mem: &File, ws_addr: usize, cs: usize, ce: usize) -> Vec<usize> {
    let mut out = vec![];

    if cs > 0 && ce > 0 {
        let ws_kids = collect_children(mem, ws_addr, cs, ce);
        for child in &ws_kids {
            if let Some(r) = rtti::scan_rtti(mem, *child) {
                if r.name == "MeshPart@RBX" {
                    out.push(*child);
                    if out.len() >= 3 { return out; }
                }
            }
            let grandkids = collect_children(mem, *child, cs, ce);
            for gk in &grandkids {
                if let Some(r) = rtti::scan_rtti(mem, *gk) {
                    if r.name == "MeshPart@RBX" {
                        out.push(*gk);
                        if out.len() >= 3 { return out; }
                    }
                }
            }
        }
    }

    if out.is_empty() {
        for off in (0..0x4000).step_by(8) {
            let ptr = match memory::read::<usize>(mem, ws_addr + off) {
                Some(p) => p,
                None => continue,
            };
            if ptr < 0x10000 { continue; }
            if let Some(r) = rtti::scan_rtti(mem, ptr) {
                if r.name == "MeshPart@RBX" {
                    out.push(ptr);
                    if out.len() >= 3 { break; }
                }
            }
        }
    }

    out
}

pub fn dump(mem: &File) -> bool {
    eprintln!("[mesh_part]");

    let ws_addr = unsafe { G_WORKSPACE_ADDR };
    let cs = G_DUMPER.get_offset("Instance", "ChildrenStart").unwrap_or(0);
    let ce = G_DUMPER.get_offset("Instance", "ChildrenEnd").unwrap_or(0);

    let mesh_parts = find_mesh_parts(mem, ws_addr, cs, ce);
    if mesh_parts.is_empty() {
        eprintln!("  No MeshPart found");
        return true;
    }
    eprintln!("  Found {} MeshPart(s)", mesh_parts.len());

    let mp = mesh_parts[0];

    // RenderFidelity: u8 enum (1=Automatic, 2=Precise, 3=Performance)
    for off in (0..0x200).step_by(1) {
        let v = match memory::read::<u8>(mem, mp + off) {
            Some(v) => v,
            None => continue,
        };
        if v >= 1 && v <= 3 {
            let v4 = match memory::read::<u32>(mem, mp + off - (off % 4)) {
                Some(v4) => v4,
                None => continue,
            };
            if (v4 & 0xFF) == v as u32 && v4 < 0x1000 {
                G_DUMPER.add_offset("MeshPart", "RenderFidelity", off);
                eprintln!("  MeshPart::RenderFidelity at +0x{:x}", off);
                break;
            }
        }
    }

    // CollisionFidelity: u8 enum (0=Default, 1=Hull, 2=Box, 3=Precise)
    for off in (0..0x200).step_by(1) {
        let v = match memory::read::<u8>(mem, mp + off) {
            Some(v) => v,
            None => continue,
        };
        if v <= 3 {
            let rf = G_DUMPER.get_offset("MeshPart", "RenderFidelity").unwrap_or(usize::MAX);
            let v4 = match memory::read::<u32>(mem, mp + off - (off % 4)) {
                Some(v4) => v4,
                None => continue,
            };
            if off != rf && (v4 & 0xFF) == v as u32 && v4 < 0x1000 {
                G_DUMPER.add_offset("MeshPart", "CollisionFidelity", off);
                eprintln!("  MeshPart::CollisionFidelity at +0x{:x}", off);
                break;
            }
        }
    }

    // MeshId: string pointer (content ID)
    for off in (0..0x400).step_by(8) {
        let ptr = match memory::read::<usize>(mem, mp + off) {
            Some(p) => p,
            None => continue,
        };
        if ptr < 0x10000 { continue; }
        if let Some(s) = memory::read_name_fmt(mem, ptr) {
            if s.starts_with("rbxasset") || s.starts_with("http") {
                G_DUMPER.add_offset("MeshPart", "MeshId", off);
                eprintln!("  MeshPart::MeshId at +0x{:x}", off);
                break;
            }
        }
        if let Some(s) = read_sso(mem, mp + off) {
            if s.starts_with("rbxasset") || s.starts_with("http") {
                G_DUMPER.add_offset("MeshPart", "MeshId", off);
                eprintln!("  MeshPart::MeshId at +0x{:x}", off);
                break;
            }
        }
    }

    // TextureId: string pointer (content ID for texture)
    for off in (0..0x400).step_by(8) {
        let ptr = match memory::read::<usize>(mem, mp + off) {
            Some(p) => p,
            None => continue,
        };
        if ptr < 0x10000 { continue; }
        let mid = G_DUMPER.get_offset("MeshPart", "MeshId").unwrap_or(usize::MAX);
        if off == mid { continue; }
        if let Some(s) = memory::read_name_fmt(mem, ptr) {
            if s.starts_with("rbxasset") || s.starts_with("http") {
                G_DUMPER.add_offset("MeshPart", "TextureId", off);
                eprintln!("  MeshPart::TextureId at +0x{:x}", off);
                break;
            }
        }
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
