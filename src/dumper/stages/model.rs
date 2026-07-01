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

fn find_models(mem: &File, ws_addr: usize, cs: usize, ce: usize) -> Vec<usize> {
    let mut out = vec![];
    if cs > 0 && ce > 0 {
        let ws_children = collect_children(mem, ws_addr, cs, ce);
        for child in &ws_children {
            if let Some(r) = rtti::scan_rtti(mem, *child) {
                if r.name == "Model@RBX" || r.name == "ModelInstance@RBX" {
                    out.push(*child);
                }
            }
        }
    }
    if out.is_empty() {
        for off in (0..0x2000).step_by(8) {
            let ptr = match memory::read::<usize>(mem, ws_addr + off) {
                Some(p) => p,
                None => continue,
            };
            if ptr < 0x10000 { continue; }
            if let Some(r) = rtti::scan_rtti(mem, ptr) {
                if r.name == "Model@RBX" || r.name == "ModelInstance@RBX" {
                    out.push(ptr);
                    if out.len() >= 3 { break; }
                }
            }
        }
    }
    out
}

pub fn dump(mem: &File) -> bool {
    eprintln!("[model]");

    let ws_addr = unsafe { G_WORKSPACE_ADDR };
    let cs = G_DUMPER.get_offset("Instance", "ChildrenStart").unwrap_or(0);
    let ce = G_DUMPER.get_offset("Instance", "ChildrenEnd").unwrap_or(0);
    let models = find_models(mem, ws_addr, cs, ce);

    // Collect all child addresses of each model
    let model_children: Vec<Vec<usize>> = models.iter().map(|&m| {
        if cs > 0 && ce > 0 {
            collect_children(mem, m, cs, ce)
        } else {
            vec![]
        }
    }).collect();

    for (idx, &model) in models.iter().enumerate() {
        if G_DUMPER.get_offset("Model", "PrimaryPart").is_some() { break; }

        // Scan model for a pointer to a BasePart that is also a child
        for off in (0..0x300).step_by(8) {
            let ptr = match memory::read::<usize>(mem, model + off) {
                Some(p) => p,
                None => continue,
            };
            if ptr < 0x10000 { continue; }

            // Check if ptr is a BasePart (has Primitive@RBX RTTI within)
            if rtti::find(mem, ptr, "Primitive@RBX", 0x1000, 8).is_some() {
                // Verify this part is a child of the model
                let is_child = model_children.get(idx)
                    .map(|kids| kids.contains(&ptr))
                    .unwrap_or(false);

                if is_child || model_children.is_empty() {
                    G_DUMPER.add_offset("Model", "PrimaryPart", off);
                    eprintln!("  Model::PrimaryPart at +0x{:x}", off);
                    break;
                }
            }
        }
    }

    true
}
