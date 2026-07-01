use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use crate::dumper::{OffsetEntry, G_DUMPER};

fn offsets_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe.parent().unwrap_or(&exe);
    dir.join("offsets")
}

fn get_val(ns: &str, name: &str) -> String {
    match G_DUMPER.get_value(ns, name) {
        Some(v) => format!(" # {}", v),
        None => String::new(),
    }
}

fn build_header(count: usize, elapsed_ms: u128) -> String {
    format!("\
// ===========================
// Turners Dumper
// Discord @grfq
// ===========================
// {} offsets in {}ms
// auto-generated, do not edit
//

", count, elapsed_ms)
}

fn write_rs(offsets: &BTreeMap<String, Vec<OffsetEntry>>, header: &str) {
    let mut out = String::from(header);
    for (ns, entries) in offsets {
        out.push_str(&format!("pub mod {} {{\n", ns));
        for e in entries {
            let v = get_val(ns, &e.name);
            out.push_str(&format!("    pub const {}: usize = 0x{:X};{}\n", e.name, e.offset, v));
        }
        out.push_str("}\n\n");
    }
    let _ = fs::write(offsets_dir().join("offsets.rs"), out.as_bytes());
}

fn write_cpp(offsets: &BTreeMap<String, Vec<OffsetEntry>>, header: &str) {
    let mut out = format!("{}#include <cstdint>\n\nnamespace offsets {{\n", header);
    for (ns, entries) in offsets {
        out.push_str(&format!("    // {}\n", ns));
        for e in entries {
            let v = get_val(ns, &e.name);
            out.push_str(&format!("    constexpr std::uintptr_t {} = 0x{:X};{}\n", e.name, e.offset, v));
        }
        out.push('\n');
    }
    out.push_str("} // namespace offsets\n");
    let _ = fs::write(offsets_dir().join("offsets.cpp"), out.as_bytes());
}

fn write_py(offsets: &BTreeMap<String, Vec<OffsetEntry>>, _header: &str) {
    let mut out = format!("{}\"\"\"\nTurners Dumper\nDiscord @grfq\n\nauto-generated, do not edit\n\"\"\"\n\n", _header);
    for (ns, entries) in offsets {
        out.push_str(&format!("# {}\n", ns));
        for e in entries {
            let v = get_val(ns, &e.name);
            out.push_str(&format!("{} = 0x{:X}{}\n", e.name, e.offset, v));
        }
        out.push('\n');
    }
    let _ = fs::write(offsets_dir().join("offsets.py"), out.as_bytes());
}

pub fn write_offsets(offsets: &BTreeMap<String, Vec<OffsetEntry>>, elapsed_ms: u128) {
    let _ = fs::create_dir_all(offsets_dir());
    let count: usize = offsets.values().map(|v| v.len()).sum();
    let header = build_header(count, elapsed_ms);

    write_rs(offsets, &header);
    write_cpp(offsets, &header);
    write_py(offsets, &header);

    let od = offsets_dir();
    println!("Found {} offsets in {}ms", count, elapsed_ms);
    println!("Written to {}/offsets.rs, {}/offsets.cpp, {}/offsets.py", od.display(), od.display(), od.display());
}
