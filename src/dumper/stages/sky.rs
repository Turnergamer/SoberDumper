use std::fs::File;
use crate::memory;
use crate::rtti;
use crate::dumper::{G_DUMPER, G_DATA_MODEL_ADDR};

pub fn dump(mem: &File) -> bool {
    eprintln!("[sky]");

    let dm_addr = unsafe { G_DATA_MODEL_ADDR };

    let lighting = rtti::find(mem, dm_addr, "Lighting@RBX", 0x2000, 8)
        .and_then(|off| memory::read::<usize>(mem, dm_addr + off));

    let lighting = match lighting {
        Some(l) => l,
        None => { eprintln!("  No Lighting found"); return true; }
    };
    eprintln!("  Lighting @ 0x{:x}", lighting);

    // Sky: find via RTTI in Lighting
    if let Some(sky_off) = rtti::find(mem, lighting, "Sky@RBX", 0x1000, 8) {
        if let Some(sky) = memory::read::<usize>(mem, lighting + sky_off) {
            if sky >= 0x10000 {
                eprintln!("  Sky @ 0x{:x}", sky);

                // CelestialBodies: find array pointer
                for off in (0..0x200).step_by(8) {
                    let ptr = match memory::read::<usize>(mem, sky + off) {
                        Some(p) => p,
                        None => continue,
                    };
                    if ptr < 0x10000 { continue; }
                    let first = memory::read::<usize>(mem, ptr).unwrap_or(0);
                    if first >= 0x10000 {
                        if let Some(r) = rtti::scan_rtti(mem, first) {
                            if r.name.contains("CelestialBody") || r.name.contains("Sun") {
                                G_DUMPER.add_offset("Sky", "CelestialBodies", off);
                                eprintln!("  Sky::CelestialBodies at +0x{:x}", off);
                                break;
                            }
                        }
                    }
                }

                // MoonAngularSize: float
                for off in (0..0x100).step_by(4) {
                    let v = match memory::read_f32(mem, sky + off) {
                        Some(v) => v,
                        None => continue,
                    };
                    if v > 0.0 && v < 90.0 && (v - 25.0).abs() < 20.0 {
                        G_DUMPER.add_offset("Sky", "MoonAngularSize", off);
                        eprintln!("  Sky::MoonAngularSize at +0x{:x} ({})", off, v);
                        break;
                    }
                }

                // SunAngularSize: float
                for off in (0..0x100).step_by(4) {
                    let v = match memory::read_f32(mem, sky + off) {
                        Some(v) => v,
                        None => continue,
                    };
                    if v > 0.0 && v < 90.0 && (v - 14.0).abs() < 10.0 {
                        let moon = G_DUMPER.get_offset("Sky", "MoonAngularSize").unwrap_or(usize::MAX);
                        if off != moon {
                            G_DUMPER.add_offset("Sky", "SunAngularSize", off);
                            eprintln!("  Sky::SunAngularSize at +0x{:x} ({})", off, v);
                            break;
                        }
                    }
                }

                // StarCount: u16 or u32
                for off in (0..0x100).step_by(2) {
                    let v = match memory::read::<u16>(mem, sky + off) {
                        Some(v) => v,
                        None => continue,
                    };
                    if v > 100 && v < 10000 {
                        let v32 = memory::read::<u32>(mem, sky + off - (off % 4)).unwrap_or(0);
                        if v32 < 100000 {
                            G_DUMPER.add_offset("Sky", "StarCount", off);
                            eprintln!("  Sky::StarCount at +0x{:x} ({})", off, v);
                            break;
                        }
                    }
                }

                // SkyboxUp, SkyboxDown, etc: string pointers to textures
                for off in (0..0x300).step_by(8) {
                    let ptr = match memory::read::<usize>(mem, sky + off) {
                        Some(p) => p,
                        None => continue,
                    };
                    if ptr < 0x10000 { continue; }
                    if let Some(s) = memory::read_name_fmt(mem, ptr) {
                        if s.contains("skybox") || s.contains("Skybox") || s.starts_with("rbxasset://") {
                            let name = if G_DUMPER.get_offset("Sky", "SkyboxUp").is_none() {
                                "SkyboxUp"
                            } else if G_DUMPER.get_offset("Sky", "SkyboxDown").is_none() {
                                "SkyboxDown"
                            } else if G_DUMPER.get_offset("Sky", "SkyboxLeft").is_none() {
                                "SkyboxLeft"
                            } else if G_DUMPER.get_offset("Sky", "SkyboxRight").is_none() {
                                "SkyboxRight"
                            } else if G_DUMPER.get_offset("Sky", "SkyboxFront").is_none() {
                                "SkyboxFront"
                            } else if G_DUMPER.get_offset("Sky", "SkyboxBack").is_none() {
                                "SkyboxBack"
                            } else {
                                continue;
                            };
                            G_DUMPER.add_offset("Sky", name, off);
                            eprintln!("  Sky::{} at +0x{:x}", name, off);
                            if G_DUMPER.get_offset("Sky", "SkyboxBack").is_some() { break; }
                        }
                    }
                }
            }
        }
    }

    // SunRaysEffect: find via RTTI in Lighting
    if let Some(sr_off) = rtti::find(mem, lighting, "SunRaysEffect@RBX", 0x1000, 8) {
        if let Some(sr) = memory::read::<usize>(mem, lighting + sr_off) {
            if sr >= 0x10000 {
                eprintln!("  SunRaysEffect @ 0x{:x}", sr);
                for off in (0..0x100).step_by(4) {
                    let v = match memory::read_f32(mem, sr + off) {
                        Some(v) => v,
                        None => continue,
                    };
                    if v >= 0.0 && v <= 1.0 {
                        if G_DUMPER.get_offset("SunRaysEffect", "Intensity").is_none() {
                            G_DUMPER.add_offset("SunRaysEffect", "Intensity", off);
                        } else if G_DUMPER.get_offset("SunRaysEffect", "Spread").is_none() {
                            G_DUMPER.add_offset("SunRaysEffect", "Spread", off);
                            break;
                        }
                    }
                }
            }
        }
    }

    // Clouds: find via RTTI
    if let Some(c_off) = rtti::find(mem, lighting, "Clouds@RBX", 0x2000, 8) {
        if let Some(c) = memory::read::<usize>(mem, lighting + c_off) {
            if c >= 0x10000 {
                eprintln!("  Clouds @ 0x{:x}", c);
                // Cover: float 0-1 (default 0.5)
                for off in (0..0x100).step_by(4) {
                    let v = match memory::read_f32(mem, c + off) {
                        Some(v) => v,
                        None => continue,
                    };
                    if (v - 0.5).abs() < 0.1 && v >= 0.0 && v <= 1.0 {
                        G_DUMPER.add_offset("Clouds", "Cover", off);
                        eprintln!("  Clouds::Cover at +0x{:x} ({})", off, v);
                        break;
                    }
                }
                // Density: float 0-1 (default 0.5)
                for off in (0..0x100).step_by(4) {
                    let v = match memory::read_f32(mem, c + off) {
                        Some(v) => v,
                        None => continue,
                    };
                    if (v - 0.5).abs() < 0.1 && v >= 0.0 && v <= 1.0 {
                        let cov = G_DUMPER.get_offset("Clouds", "Cover").unwrap_or(usize::MAX);
                        if off != cov {
                            G_DUMPER.add_offset("Clouds", "Density", off);
                            eprintln!("  Clouds::Density at +0x{:x} ({})", off, v);
                            break;
                        }
                    }
                }
            }
        }
    }

    true
}
