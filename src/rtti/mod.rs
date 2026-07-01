use crate::memory;
use std::fs::File;

pub struct RttiInfo {
    pub name: String,
}

fn is_valid_ptr(ptr: usize) -> bool {
    ptr >= 0x10000 && ptr <= 0x7fffffffffff
}

fn demangle_itanium(mangled: &str) -> String {
    if mangled.is_empty() || !mangled.starts_with('N') {
        return mangled.to_string();
    }

    let mut components: Vec<String> = vec![];
    let mut pos = 1;

    let bytes = mangled.as_bytes();
    while pos < bytes.len() && bytes[pos] != b'E' {
        if !bytes[pos].is_ascii_digit() {
            return mangled.to_string();
        }

        let start = pos;
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            pos += 1;
        }
        let len: usize = match mangled[start..pos].parse() {
            Ok(l) => l,
            Err(_) => return mangled.to_string(),
        };
        if len == 0 || len > 256 { return mangled.to_string(); }

        if pos + len > mangled.len() { return mangled.to_string(); }
        components.push(mangled[pos..pos+len].to_string());
        pos += len;
    }

    if components.is_empty() { return mangled.to_string(); }

    let mut result = String::new();
    for i in (0..components.len()).rev() {
        if !result.is_empty() { result.push('@'); }
        result.push_str(&components[i]);
    }
    result
}

/// Scan RTTI at a given address (3 memory reads).
pub fn scan_rtti(mem: &File, address: usize) -> Option<RttiInfo> {
    let vtable_ptr = memory::read::<usize>(mem, address)?;
    if !is_valid_ptr(vtable_ptr) { return None; }

    let typeinfo_ptr = memory::read::<usize>(mem, vtable_ptr.wrapping_sub(8))?;
    if !is_valid_ptr(typeinfo_ptr) { return None; }

    let name_ptr = memory::read::<usize>(mem, typeinfo_ptr + 8)?;
    if !is_valid_ptr(name_ptr) { return None; }

    let mangled = memory::read_string(mem, name_ptr, 256)?;
    if mangled.is_empty() { return None; }

    let first = mangled.as_bytes()[0];
    if !first.is_ascii_alphanumeric() && first != b'N' && first != b'Z' {
        return None;
    }

    Some(RttiInfo { name: demangle_itanium(&mangled) })
}

/// Scan a memory region (chunk of bytes) for valid pointers and check RTTI on each.
/// Calls `on_match` for every pointer whose RTTI name matches `target_class`.
/// Uses pread64 in large batches for speed.
pub fn scan_section_batched(
    mem: &File,
    section_start: usize,
    section_size: usize,
    module_base: usize,
    alignment: usize,
    on_match: &mut dyn FnMut(usize, &str),
) {
    let page_size: usize = 1024 * 1024; // 1MB per batch read
    let stride = alignment;

    let mut page_buf = vec![0u8; page_size];

    let mut offset: usize = 0;
    while offset < section_size {
        let remaining = section_size - offset;
        let to_read = std::cmp::min(page_size, remaining);
        let chunk = &mut page_buf[..to_read];

        let addr = section_start + offset;
        let n = match memory::read_into(mem, addr, chunk) {
            Some(n) => n,
            None => { offset += page_size; continue; }
        };

        // Align offset to stride within the chunk
        let chunk_end = (n / stride) * stride;
        let mut i = 0;
        while i + 8 <= chunk_end {
            let ptr_val = u64::from_ne_bytes([
                chunk[i], chunk[i+1], chunk[i+2], chunk[i+3],
                chunk[i+4], chunk[i+5], chunk[i+6], chunk[i+7],
            ]) as usize;

            if is_valid_ptr(ptr_val) {
                if let Some(rtti) = scan_rtti(mem, ptr_val) {
                    let module_off = (addr + i) - module_base;
                    (on_match)(module_off, &rtti.name);
                }
            }
            i += stride;
        }

        offset += page_size;
    }
}

/// Find first RTTI match in range (small range, individual reads)
pub fn find(mem: &File, base_address: usize, target_class: &str,
            max_offset: usize, alignment: usize) -> Option<usize> {
    let mut offset = 0;
    while offset < max_offset {
        let addr = base_address + offset;
        let ptr_val = memory::read::<usize>(mem, addr)?;
        if !is_valid_ptr(ptr_val) { offset += alignment; continue; }

        if let Some(rtti) = scan_rtti(mem, ptr_val) {
            if rtti.name == target_class {
                return Some(offset);
            }
        }
        offset += alignment;
    }
    None
}
