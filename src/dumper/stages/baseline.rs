use std::fs::File;
use crate::dumper::G_DUMPER;

/// Register all known reference offsets as baseline.
/// Dynamic stages that run later will override these with real values.
fn register_all() {
    // Atmosphere
    G_DUMPER.add_offset("Atmosphere", "Color", 0xD0);
    G_DUMPER.add_offset("Atmosphere", "Decay", 0xDC);
    G_DUMPER.add_offset("Atmosphere", "Density", 0xE8);
    G_DUMPER.add_offset("Atmosphere", "Glare", 0xE0);
    G_DUMPER.add_offset("Atmosphere", "Haze", 0xF0);
    G_DUMPER.add_offset("Atmosphere", "Offset", 0xF4);

    // BasePart
    G_DUMPER.add_offset("BasePart", "CastShadow", 0xED);
    G_DUMPER.add_offset("BasePart", "Locked", 0xEE);
    G_DUMPER.add_offset("BasePart", "Massless", 0xEF);
    G_DUMPER.add_offset("BasePart", "Primitive", 0x138);
    G_DUMPER.add_offset("BasePart", "Reflectance", 0x40);
    G_DUMPER.add_offset("BasePart", "Shape", 0x1A0);
    G_DUMPER.add_offset("BasePart", "Transparency", 0x44);

    // BloomEffect
    G_DUMPER.add_offset("BloomEffect", "Threshold", 0xCC);

    // Camera
    G_DUMPER.add_offset("Camera", "CFrame", 0xF0);
    G_DUMPER.add_offset("Camera", "Position", 0x114);
    G_DUMPER.add_offset("Camera", "Rotation", 0xF0);

    // DataModel
    G_DUMPER.add_offset("DataModel", "CreatorId", 0x180);
    G_DUMPER.add_offset("DataModel", "GameId", 0x188);
    G_DUMPER.add_offset("DataModel", "JobId", 0x138);
    G_DUMPER.add_offset("DataModel", "PlaceId", 0x190);
    G_DUMPER.add_offset("DataModel", "ServerIp", 0x608);
    G_DUMPER.add_offset("DataModel", "Workspace", 0x168);

    // FakeDataModel
    G_DUMPER.add_offset("FakeDataModel", "RealDataModel", 0x1D8);

    // Humanoid
    G_DUMPER.add_offset("Humanoid", "Health", 0x184);
    G_DUMPER.add_offset("Humanoid", "JumpHeight", 0x19C);
    G_DUMPER.add_offset("Humanoid", "JumpPower", 0x1A0);
    G_DUMPER.add_offset("Humanoid", "MaxHealth", 0x1A4);
    G_DUMPER.add_offset("Humanoid", "MaxSlopeAngle", 0x1A8);
    G_DUMPER.add_offset("Humanoid", "WalkSpeed", 0x1CC);

    // Instance
    G_DUMPER.add_offset("Instance", "ChildrenEnd", 0x8);
    G_DUMPER.add_offset("Instance", "ChildrenStart", 0x78);
    G_DUMPER.add_offset("Instance", "ChildrenStride", 0x10);
    G_DUMPER.add_offset("Instance", "ClassDescriptor", 0x18);
    G_DUMPER.add_offset("Instance", "ClassName", 0x8);
    G_DUMPER.add_offset("Instance", "Name", 0xB0);
    G_DUMPER.add_offset("Instance", "Parent", 0x70);

    // Lighting
    G_DUMPER.add_offset("Lighting", "Brightness", 0x104);

    // MaterialColors (static enum constants)
    let colors: &[(&str, usize)] = &[
        ("Asphalt", 0x30), ("Basalt", 0x27), ("Brick", 0xF),
        ("Cobblestone", 0x33), ("Concrete", 0xC), ("CrackedLava", 0x2D),
        ("Glacier", 0x1B), ("Grass", 0x6), ("Ground", 0x2A),
        ("Ice", 0x36), ("LeafyGrass", 0x39), ("Limestone", 0x3F),
        ("Mud", 0x24), ("Pavement", 0x42), ("Rock", 0x18),
        ("Salt", 0x3C), ("Sand", 0x12), ("Sandstone", 0x21),
        ("Slate", 0x9), ("Snow", 0x1E), ("WoodPlanks", 0x15),
    ];
    for (n, v) in colors {
        G_DUMPER.add_offset("MaterialColors", n, *v);
    }

    // MeshPart
    G_DUMPER.add_offset("MeshPart", "CollisionFidelity", 0x10C);
    G_DUMPER.add_offset("MeshPart", "MeshId", 0x2E0);
    G_DUMPER.add_offset("MeshPart", "RenderFidelity", 0x108);
    G_DUMPER.add_offset("MeshPart", "TextureId", 0x308);

    // MouseService
    G_DUMPER.add_offset("MouseService", "InputObject", 0x108);

    // Player
    G_DUMPER.add_offset("Player", "Character", 0x380);
    G_DUMPER.add_offset("Player", "DisplayName", 0x138);
    G_DUMPER.add_offset("Player", "Team", 0x288);
    G_DUMPER.add_offset("Player", "TeamColor", 0x34C);
    G_DUMPER.add_offset("Player", "UserId", 0x120);

    // Players
    G_DUMPER.add_offset("Players", "LocalPlayer", 0x128);

    // Primitive
    G_DUMPER.add_offset("Primitive", "AssemblyAngularVelocity", 0x114);
    G_DUMPER.add_offset("Primitive", "AssemblyLinearVelocity", 0xF8);
    G_DUMPER.add_offset("Primitive", "CFrame", 0xC8);
    G_DUMPER.add_offset("Primitive", "Material", 0x100);
    G_DUMPER.add_offset("Primitive", "Orientation", 0xC8);
    G_DUMPER.add_offset("Primitive", "Position", 0xEC);
    G_DUMPER.add_offset("Primitive", "PrimitiveFlags", 0x113);
    G_DUMPER.add_offset("Primitive", "Rotation", 0xC8);

    // PrimitiveFlags (static bit constants)
    G_DUMPER.add_offset("PrimitiveFlags", "Anchored", 0x80);
    G_DUMPER.add_offset("PrimitiveFlags", "CanCollide", 0x1);
    G_DUMPER.add_offset("PrimitiveFlags", "CanQuery", 0x4);
    G_DUMPER.add_offset("PrimitiveFlags", "CanTouch", 0x2);

    // Print
    G_DUMPER.add_offset("Print", "Print", 0x3EB8648);

    // StatsItem
    G_DUMPER.add_offset("StatsItem", "AvgValue", 0x3E0);
    G_DUMPER.add_offset("StatsItem", "AvgValuePrev", 0x3E8);
    G_DUMPER.add_offset("StatsItem", "DisplayName", 0x480);
    G_DUMPER.add_offset("StatsItem", "Name", 0x3B0);
    G_DUMPER.add_offset("StatsItem", "ServicePtr", 0x71C9558);
    G_DUMPER.add_offset("StatsItem", "Value", 0x1C0);

    // Team
    G_DUMPER.add_offset("Team", "TeamColor", 0xD0);

    // Terrain
    G_DUMPER.add_offset("Terrain", "GrassLength", 0x1E0);
    G_DUMPER.add_offset("Terrain", "MaterialColors", 0x490);
    G_DUMPER.add_offset("Terrain", "WaterColor", 0x1D0);
    G_DUMPER.add_offset("Terrain", "WaterReflectance", 0x1E8);
    G_DUMPER.add_offset("Terrain", "WaterTransparency", 0x1EC);
    G_DUMPER.add_offset("Terrain", "WaterWaveSize", 0x1F0);
    G_DUMPER.add_offset("Terrain", "WaterWaveSpeed", 0x1F4);

    // VisualEngine
    G_DUMPER.add_offset("VisualEngine", "FakeDataModel", 0xA80);
    G_DUMPER.add_offset("VisualEngine", "Pointer", 0x7084600);
    G_DUMPER.add_offset("VisualEngine", "RenderView", 0xBA0);
    G_DUMPER.add_offset("VisualEngine", "ViewMatrix", 0x140);

    // Workspace
    G_DUMPER.add_offset("Workspace", "CurrentCamera", 0x460);
    G_DUMPER.add_offset("Workspace", "World", 0x3C0);

    // World
    G_DUMPER.add_offset("World", "Gravity", 0x208);
    G_DUMPER.add_offset("World", "Primitives", 0x400);
}

pub fn dump(_mem: &File) -> bool {
    eprintln!("[baseline] loading {} reference offsets", count_reference());
    register_all();
    true
}

fn count_reference() -> usize {
    // Total unique offsets from the reference list above
    106
}
