use std::fs::File;
use crate::memory;
use crate::rtti;
use crate::dumper::{G_DUMPER, collect_children};

fn find_service(mem: &File, dm_addr: usize, class_name: &str) -> Option<usize> {
    for &c in &collect_children(mem, dm_addr) {
        if let Some(r) = rtti::scan_rtti(mem, c) {
            if r.name == class_name { return Some(c); }
        }
    }
    for off in (0..0x4000).step_by(8) {
        let ptr = match memory::read::<usize>(mem, dm_addr + off) {
            Some(p) => p,
            None => continue,
        };
        if ptr < 0x10000 { continue; }
        if let Some(r) = rtti::scan_rtti(mem, ptr) {
            if r.name == class_name { return Some(ptr); }
        }
    }
    None
}

fn dump_sound_service(mem: &File, svc: usize) {
    for off in (0..0x200).step_by(4) {
        if let Some(v) = memory::read_f32(mem, svc + off) {
            if v >= 0.0 && v <= 2.0 {
                G_DUMPER.add_offset("SoundService", "Volume", off);
                break;
            }
        }
    }
}

fn dump_user_input_service(mem: &File, svc: usize) {
    for off in (0..0x200).step_by(4) {
        if let Some(v) = memory::read::<u32>(mem, svc + off) {
            if v >= 1 && v <= 3 {
                let next = memory::read::<u32>(mem, svc + off + 4).unwrap_or(99);
                if next > 10 {
                    G_DUMPER.add_offset("UserInputService", "MouseBehavior", off);
                    break;
                }
            }
        }
    }
    for off in (0..0x200).step_by(4) {
        if let Some(v) = memory::read_f32(mem, svc + off) {
            if (v - 1.0).abs() < 0.1 && v > 0.0 {
                G_DUMPER.add_offset("UserInputService", "MouseDeltaSensitivity", off);
                break;
            }
        }
    }
    for off in (0..0x100).step_by(1) {
        if let Some(v) = memory::read::<u8>(mem, svc + off) {
            if v == 1 {
                let off_name = if G_DUMPER.get_offset("UserInputService", "MouseEnabled").is_none() {
                    "MouseEnabled"
                } else if G_DUMPER.get_offset("UserInputService", "KeyboardEnabled").is_none() {
                    "KeyboardEnabled"
                } else if G_DUMPER.get_offset("UserInputService", "TouchEnabled").is_none() {
                    "TouchEnabled"
                } else { continue; };
                G_DUMPER.add_offset("UserInputService", off_name, off);
            }
        }
    }
}

fn dump_core_gui(mem: &File, svc: usize) {
    for &k in &collect_children(mem, svc) {
        if let Some(r) = rtti::scan_rtti(mem, k) {
            if r.name == "ScreenGui@RBX" {
                for off in (0..0x200).step_by(8) {
                    if let Some(ptr) = memory::read::<usize>(mem, k + off) {
                        if ptr < 0x10000 { continue; }
                        if let Some(s) = memory::read_name_fmt(mem, ptr) {
                            if !s.is_empty() && s.len() < 60 {
                                let ns = format!("ScreenGui_{}", s);
                                G_DUMPER.add_offset(&ns, "Address", k);
                                break;
                            }
                        }
                    }
                    if let Some(s) = read_sso(mem, k + off) {
                        if !s.is_empty() && s.len() < 60 {
                            let ns = format!("ScreenGui_{}", s);
                            G_DUMPER.add_offset(&ns, "Address", k);
                            break;
                        }
                    }
                }
            }
        }
    }
}

fn dump_network_client(mem: &File, svc: usize) {
    for off in (0..0x200).step_by(4) {
        if let Some(v) = memory::read::<u32>(mem, svc + off) {
            if v <= 4 {
                G_DUMPER.add_offset("NetworkClient", "ConnectionState", off);
                break;
            }
        }
    }
    for off in (0..0x300).step_by(8) {
        if let Some(ptr) = memory::read::<usize>(mem, svc + off) {
            if ptr < 0x10000 { continue; }
            if let Some(s) = memory::read_name_fmt(mem, ptr) {
                if s.len() > 30 && (s.contains('-') || s.contains('_')) {
                    G_DUMPER.add_offset("NetworkClient", "Ticket", off);
                    break;
                }
            }
        }
    }
}

fn dump_run_service(mem: &File, svc: usize) {
    for off in (0..0x200).step_by(4) {
        if let Some(v) = memory::read_f32(mem, svc + off) {
            if v > 0.005 && v < 0.1 {
                G_DUMPER.add_offset("RunService", "HeartbeatDelta", off);
                break;
            }
        }
    }
    for off in (0..0x200).step_by(4) {
        if let Some(v) = memory::read_f32(mem, svc + off) {
            if v > 0.005 && v < 0.1 {
                let hb = G_DUMPER.get_offset("RunService", "HeartbeatDelta").unwrap_or(usize::MAX);
                if off != hb {
                    G_DUMPER.add_offset("RunService", "RenderSteppedDelta", off);
                    break;
                }
            }
        }
    }
    for off in (0..0x200).step_by(4) {
        if let Some(v) = memory::read_f32(mem, svc + off) {
            if (v - 240.0).abs() < 10.0 {
                G_DUMPER.add_offset("RunService", "SimulationRate", off);
                break;
            }
        }
    }
}

fn dump_replicated_storage(mem: &File, svc: usize) {
    let kids = collect_children(mem, svc);
    eprintln!("  ReplicatedStorage has {} child(ren)", kids.len());
}

fn dump_script_context(mem: &File, svc: usize) {
    for off in (0..0x500).step_by(8) {
        if let Some(ptr) = memory::read::<usize>(mem, svc + off) {
            if ptr < 0x10000 { continue; }
            if let Some(r) = rtti::scan_rtti(mem, ptr) {
                if r.name.contains("Script") || r.name.contains("ExecutionContext") || r.name.contains("Vm") {
                    G_DUMPER.add_offset("ScriptContext", &format!("{}Ref", r.name.split('@').next().unwrap_or("Unknown")), off);
                    break;
                }
            }
        }
    }
    for off in (0..0x200).step_by(8) {
        if let Some(v) = memory::read::<usize>(mem, svc + off) {
            if v > 100_000_000 && v < 1_000_000_000_000 {
                G_DUMPER.add_offset("ScriptContext", "ExtraMemory", off);
                break;
            }
        }
    }
}

pub fn dump(mem: &File) -> bool {
    eprintln!("[services_extra]");

    let dm_addr = unsafe { crate::dumper::G_DATA_MODEL_ADDR };

    let service_todos: &[(&str, fn(&File, usize))] = &[
        ("SoundService@RBX",      dump_sound_service),
        ("UserInputService@RBX",  dump_user_input_service),
        ("CoreGui@RBX",           dump_core_gui),
        ("NetworkClient@RBX",     dump_network_client),
        ("RunService@RBX",        dump_run_service),
        ("ReplicatedStorage@RBX", dump_replicated_storage),
        ("ScriptContext@RBX",     dump_script_context),
    ];

    for &(rtti_name, dumper_fn) in service_todos {
        if let Some(addr) = find_service(mem, dm_addr, rtti_name) {
            let ns = rtti_name.split('@').next().unwrap_or("Service");
            eprintln!("  {} @ 0x{:x}", ns, addr);
            G_DUMPER.add_offset(ns, "Address", addr);
            dumper_fn(mem, addr);
        }
    }

    let extra_services = &[
        "ReplicatedFirst@RBX",
        "ServerScriptService@RBX",
        "ServerStorage@RBX",
        "StarterGui@RBX",
        "StarterPack@RBX",
        "StarterPlayer@RBX",
        "HttpService@RBX",
        "LogService@RBX",
        "Chat@RBX",
        "InsertService@RBX",
        "ContentProvider@RBX",
        "Debris@RBX",
        "CollectionService@RBX",
        "PhysicsService@RBX",
        "JointsService@RBX",
        "TweenService@RBX",
        "Selection@RBX",
        "GuiService@RBX",
        "Teams@RBX",
        "BadgeService@RBX",
        "SocialService@RBX",
        "MarketplaceService@RBX",
        "TeleportService@RBX",
        "PresenceService@RBX",
        "LocalizationService@RBX",
        "PolicyService@RBX",
        "FriendService@RBX",
        "GroupService@RBX",
        "SuggestionsService@RBX",
        "PermissionsService@RBX",
        "RobloxReplicatedStorage@RBX",
        "RbxAnalyticsService@RBX",
        "AnalyticsService@RBX",
        "AdService@RBX",
        "VRService@RBX",
        "VoiceService@RBX",
        "TextChatService@RBX",
        "GamepadService@RBX",
        "KeyframeSequenceProvider@RBX",
        "AnimationClipProvider@RBX",
        "MaterialService@RBX",
        "TerrainRegion@RBX",
    ];

    for &rtti_name in extra_services {
        let ns = rtti_name.split('@').next().unwrap_or("Service");
        if G_DUMPER.get_offset(ns, "Address").is_some() { continue; }
        if let Some(addr) = find_service(mem, dm_addr, rtti_name) {
            eprintln!("  {} @ 0x{:x}", ns, addr);
            G_DUMPER.add_offset(ns, "Address", addr);
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
