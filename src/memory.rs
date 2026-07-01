use std::fs;
use std::os::fd::AsRawFd;

pub fn read<T: Copy + Default>(mem: &fs::File, address: usize) -> Option<T> {
    let mut val: T = Default::default();
    let size = std::mem::size_of::<T>();
    let buf = unsafe {
        std::slice::from_raw_parts_mut(&mut val as *mut T as *mut u8, size)
    };
    pread_exact(mem, buf, address).ok()?;
    Some(val)
}

pub fn read_bytes(mem: &fs::File, address: usize, size: usize) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; size];
    pread_exact(mem, &mut buf, address).ok()?;
    Some(buf)
}

pub fn read_into(mem: &fs::File, address: usize, buf: &mut [u8]) -> Option<usize> {
    let fd = mem.as_raw_fd();
    let n = unsafe {
        libc::pread64(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len(), address as i64)
    };
    if n >= 0 { Some(n as usize) } else { None }
}

pub fn read_string(mem: &fs::File, address: usize, max_len: usize) -> Option<String> {
    let buf = read_bytes(mem, address, max_len)?;
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    let s = String::from_utf8_lossy(&buf[..end]).to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// Roblox Name format: byte 0 = (length << 1) | is_long
pub fn read_name_fmt(mem: &fs::File, address: usize) -> Option<String> {
    let ctrl = read::<u8>(mem, address)?;
    let is_long = (ctrl & 1) != 0;
    let len = (ctrl >> 1) as usize;
    if len == 0 { return None; }
    if is_long {
        let ptr = read::<usize>(mem, address)?;
        let ptr_cleared = ptr & !1;
        read_string(mem, ptr_cleared, len)
    } else {
        read_string(mem, address + 1, len)
    }
}

/// Read f32, return None if NaN, Inf, or subnormal
pub fn read_f32(mem: &fs::File, address: usize) -> Option<f32> {
    let v: f32 = read(mem, address)?;
    if v.is_nan() || v.is_infinite() || v.is_subnormal() { None } else { Some(v) }
}

/// Read [f32; N], return None if any element is invalid
pub fn read_f32x3(mem: &fs::File, address: usize) -> Option<[f32; 3]> {
    let v: [f32; 3] = read(mem, address)?;
    for &x in &v {
        if x.is_nan() || x.is_infinite() || x.is_subnormal() { return None; }
    }
    Some(v)
}

/// Check that an f32 is not NaN, Inf, or subnormal
pub fn check_f32(v: f32) -> bool {
    !v.is_nan() && !v.is_infinite() && !v.is_subnormal()
}

/// Read any Copy type via pread
pub fn read_any<T: Copy>(mem: &fs::File, address: usize) -> Option<T> {
    let mut val: T = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<T>();
    let buf = unsafe {
        std::slice::from_raw_parts_mut(&mut val as *mut T as *mut u8, size)
    };
    pread_exact(mem, buf, address).ok()?;
    Some(val)
}

/// Read i64, return None if value is unreasonable for an offset field
pub fn read_i64_sane(mem: &fs::File, address: usize) -> Option<i64> {
    let v: i64 = read(mem, address)?;
    // Reject if it looks like a pointer or garbage
    if v < -10_000_000 || v > 10_000_000 { None } else { Some(v) }
}

fn pread_exact(mem: &fs::File, buf: &mut [u8], address: usize) -> std::io::Result<()> {
    let fd = mem.as_raw_fd();
    let n = unsafe {
        libc::pread64(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len(), address as i64)
    };
    if n as usize == buf.len() { Ok(()) } else { Err(std::io::Error::last_os_error()) }
}
