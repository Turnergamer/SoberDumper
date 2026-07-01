use std::fs::File;
use crate::memory;
use crate::rtti;
use crate::dumper::{G_DUMPER, G_WORKSPACE_ADDR};

pub fn dump(mem: &File) -> bool {
    eprintln!("[camera]");

    let ws_addr = unsafe { G_WORKSPACE_ADDR };
    let cc_off = G_DUMPER.get_offset("Workspace", "CurrentCamera")
        .expect("No CurrentCamera offset");
    let cam_addr = memory::read::<usize>(mem, ws_addr + cc_off)
        .expect("Failed to read Camera addr");
    eprintln!("  Camera @ 0x{:x}", cam_addr);

    // CFrame: find orthonormal rotation matrix + translation
    let mut cframe_found = false;
    for off in (0..0x300).step_by(4) {
        let buf = match memory::read_bytes(mem, cam_addr + off, 48) {
            Some(b) => b,
            None => continue,
        };
        let f: &[f32; 12] = unsafe { &*(buf.as_ptr() as *const [f32; 12]) };
        if f.iter().any(|v| v.is_nan() || v.is_infinite()) { continue; }

        let axes: [[f32; 3]; 3] = [[f[0], f[1], f[2]], [f[3], f[4], f[5]], [f[6], f[7], f[8]]];
        let mut ortho = true;
        for a in &axes {
            let len = (a[0]*a[0] + a[1]*a[1] + a[2]*a[2]).sqrt();
            if (len - 1.0).abs() > 0.01 || a[0].abs() > 1.5 || a[1].abs() > 1.5 || a[2].abs() > 1.5 {
                ortho = false; break;
            }
        }
        if !ortho { continue; }
        let dot01 = axes[0][0]*axes[1][0]+axes[0][1]*axes[1][1]+axes[0][2]*axes[1][2];
        let dot02 = axes[0][0]*axes[2][0]+axes[0][1]*axes[2][1]+axes[0][2]*axes[2][2];
        if dot01.abs() > 0.01 || dot02.abs() > 0.01 { continue; }

        let det = axes[0][0]*(axes[1][1]*axes[2][2]-axes[1][2]*axes[2][1])
                - axes[0][1]*(axes[1][0]*axes[2][2]-axes[1][2]*axes[2][0])
                + axes[0][2]*(axes[1][0]*axes[2][1]-axes[1][1]*axes[2][0]);
        if (det - 1.0).abs() > 0.02 { continue; }

        let pos = [f[9], f[10], f[11]];
        if pos[0].abs() > 1e8 || pos[1].abs() > 1e8 || pos[2].abs() > 1e8 { continue; }

        G_DUMPER.add_offset("Camera", "CFrame", off);
        G_DUMPER.add_offset("Camera", "Position", off + 0x24);
        G_DUMPER.add_offset("Camera", "Rotation", off);
        eprintln!("  CFrame at +0x{:x}", off);
        cframe_found = true;
        break;
    }
    if !cframe_found { return false; }

    // CameraSubject: find pointer to Humanoid via RTTI
    for off in (0..0x200).step_by(8) {
        let ptr = match memory::read::<usize>(mem, cam_addr + off) {
            Some(p) => p,
            None => continue,
        };
        if ptr < 0x10000 { continue; }
        if let Some(rtti) = rtti::scan_rtti(mem, ptr) {
            if rtti.name == "Humanoid@RBX" {
                G_DUMPER.add_offset("Camera", "CameraSubject", off);
                eprintln!("  CameraSubject at +0x{:x}", off);
                break;
            }
        }
    }

    // CameraType: uint32 = 1 (enum Custom)
    for off in (0..0x100).step_by(4) {
        let v = match memory::read::<u32>(mem, cam_addr + off) {
            Some(v) => v,
            None => continue,
        };
        if v == 1 {
            let next = memory::read::<u32>(mem, cam_addr + off + 4).unwrap_or(99);
            if next > 10 { // not part of a larger field
                G_DUMPER.add_offset("Camera", "CameraType", off);
                break;
            }
        }
    }

    // Viewport: scan for vec2 matching display resolution or plausible dims
    let common_res: [(f32, f32); 8] = [
        (1920.0, 1080.0), (1366.0, 768.0), (2560.0, 1440.0),
        (3840.0, 2160.0), (1440.0, 900.0), (1536.0, 864.0),
        (1280.0, 720.0), (1680.0, 1050.0),
    ];

    // Try vec2 scan first
    for off in (0..0x500).step_by(4) {
        let v = match memory::read::<[f32; 2]>(mem, cam_addr + off) {
            Some(v) => v,
            None => continue,
        };
        if v[0].is_nan() || v[1].is_nan() || v[0].is_infinite() || v[1].is_infinite() || v[0].is_subnormal() || v[1].is_subnormal() { continue; }
        if v[0] < 100.0 || v[1] < 100.0 || v[0] > 10000.0 || v[1] > 10000.0 { continue; }
        let aspect = v[0] / v[1];
        if aspect < 0.5 || aspect > 3.0 { continue; }

        let mut matched = false;
        for &(rw, rh) in &common_res {
            if (v[0] - rw).abs() < 10.0 && (v[1] - rh).abs() < 10.0 { matched = true; break; }
            if (v[0] - rh).abs() < 10.0 && (v[1] - rw).abs() < 10.0 { matched = true; break; }
        }
        if matched || (v[0] > 100.0 && v[0] < 10000.0 && aspect > 0.5 && aspect < 3.0) {
            G_DUMPER.add_offset("Camera", "ViewportSize", off);
            eprintln!("  ViewportSize at +0x{:x} ({:.0}x{:.0})", off, v[0], v[1]);
            break;
        }
    }

    // Try int16 scan for Viewport
    if G_DUMPER.get_offset("Camera", "ViewportSize").is_none() {
        for off in (0..0x500).step_by(2) {
            let x = match memory::read::<i16>(mem, cam_addr + off) {
                Some(x) => x,
                None => continue,
            };
            let y = match memory::read::<i16>(mem, cam_addr + off + 2) {
                Some(y) => y,
                None => continue,
            };
            if x > 100 && y > 100 && x < 10000 && y < 10000 {
                let aspect = x as f32 / y as f32;
                if aspect > 0.5 && aspect < 3.0 {
                    G_DUMPER.add_offset("Camera", "ViewportSize", off);
                    eprintln!("  ViewportSize at +0x{:x} (int16: {}x{})", off, x, y);
                    break;
                }
            }
        }
    }

    true
}
