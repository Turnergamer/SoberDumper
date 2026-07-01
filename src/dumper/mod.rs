use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Clone)]
pub struct OffsetEntry {
    pub name: String,
    pub offset: usize,
}

pub struct Dumper {
    pub offsets: Mutex<BTreeMap<String, Vec<OffsetEntry>>>,
    pub values: Mutex<BTreeMap<String, u64>>,
}

impl Dumper {
    pub fn new() -> Self {
        Self { offsets: Mutex::new(BTreeMap::new()), values: Mutex::new(BTreeMap::new()) }
    }

    pub fn add_offset(&self, ns: &str, name: &str, offset: usize) {
        let mut map = self.offsets.lock().unwrap();
        let entries = map.entry(ns.to_string()).or_default();
        if let Some(existing) = entries.iter_mut().find(|e| e.name == name) {
            existing.offset = offset;
        } else {
            entries.push(OffsetEntry { name: name.to_string(), offset });
        }
    }

    pub fn add_offset_val(&self, ns: &str, name: &str, offset: usize, value: u64) {
        self.add_offset(ns, name, offset);
        let key = format!("{}::{}", ns, name);
        self.values.lock().unwrap().insert(key, value);
    }

    pub fn get_offset(&self, ns: &str, name: &str) -> Option<usize> {
        let map = self.offsets.lock().unwrap();
        if let Some(entries) = map.get(ns) {
            for e in entries {
                if e.name == name { return Some(e.offset); }
            }
        }
        None
    }

    pub fn get_value(&self, ns: &str, name: &str) -> Option<u64> {
        let key = format!("{}::{}", ns, name);
        self.values.lock().unwrap().get(&key).copied()
    }
}

pub mod stages {
    pub mod baseline;
    pub mod visual_engine;
    pub mod data_model;
    pub mod instance;
    pub mod workspace;
    pub mod camera;
    pub mod player;
    pub mod base_part;
    pub mod humanoid;
    pub mod model;
    pub mod lighting;
    pub mod mesh_part;
    pub mod constants;
    pub mod services_extra;
    pub mod part_details;
    pub mod humanoid_details;
    pub mod sound;
    pub mod attachment;
    pub mod humanoid_ext;
    pub mod datamodel_ext;
    pub mod sky;
    pub mod character_ext;
}

use std::sync::LazyLock;
pub static G_DUMPER: LazyLock<Dumper> = LazyLock::new(|| Dumper::new());
pub static mut G_VISUAL_ENGINE: usize = 0;
pub static mut G_DATA_MODEL_ADDR: usize = 0;
pub static mut G_WORKSPACE_ADDR: usize = 0;

/// Walk Instance children linked list. Returns all direct child instance addresses.
pub fn collect_children(mem: &std::fs::File, addr: usize) -> Vec<usize> {
    let mut out = vec![];
    let cs = G_DUMPER.get_offset("Instance", "ChildrenStart").unwrap_or(0);
    let ce = G_DUMPER.get_offset("Instance", "ChildrenEnd").unwrap_or(0);
    if cs == 0 || ce == 0 { return out; }
    let llist = crate::memory::read::<usize>(mem, addr + cs).unwrap_or(0);
    if llist < 0x10000 { return out; }
    let sentinel = crate::memory::read::<usize>(mem, llist + ce).unwrap_or(0);
    if sentinel < 0x10000 { return out; }
    let first_node = crate::memory::read::<usize>(mem, llist).unwrap_or(0);
    if first_node < 0x10000 { return out; }
    let mut node = first_node;
    for _ in 0..500 {
        if node == sentinel || node == 0 { break; }
        if let Some(child) = crate::memory::read::<usize>(mem, node) {
            if child >= 0x10000 { out.push(child); }
        }
        node += 0x10;
    }
    out
}

/// Recursively find all Instance descendants matching a given RTTI class name.
pub fn find_instances(mem: &std::fs::File, root: usize, class: &str, max: usize) -> Vec<usize> {
    let mut out = vec![];
    let mut stack = vec![root];
    while let Some(addr) = stack.pop() {
        if out.len() >= max { break; }
        let kids = collect_children(mem, addr);
        for &k in &kids {
            if let Some(r) = crate::rtti::scan_rtti(mem, k) {
                if r.name == class { out.push(k); if out.len() >= max { break; } }
            }
            stack.push(k);
        }
    }
    out
}
