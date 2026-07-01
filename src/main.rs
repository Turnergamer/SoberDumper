mod process;
mod memory;
mod rtti;
mod dumper;
mod writer;

use std::time::Instant;
use dumper::{G_DUMPER, stages};

fn main() {
    println!("==============================");
    println!("    Turners Dumper");
    println!("    Discord @grfq");
    println!("==============================");
    println!();

    let mut proc = process::Process::new();
    if !proc.attach("Main") {
        std::process::exit(1);
    }

    let mem = proc.mem_file();
    let start = Instant::now();

    stages::baseline::dump(mem);
    if !stages::visual_engine::dump(&proc, mem) { std::process::exit(1); }
    if !stages::data_model::dump(mem) { std::process::exit(1); }
    if !stages::instance::dump(mem) { std::process::exit(1); }
    if !stages::workspace::dump(mem) { std::process::exit(1); }
    if !stages::camera::dump(mem) { std::process::exit(1); }
    stages::player::dump(mem);
    stages::base_part::dump(mem);
    stages::humanoid::dump(mem);
    stages::model::dump(mem);
    stages::lighting::dump(mem);
    stages::mesh_part::dump(mem);
    stages::constants::dump(mem);
    stages::services_extra::dump(mem);
    stages::part_details::dump(mem);
    stages::humanoid_details::dump(mem);
    stages::sound::dump(mem);
    stages::attachment::dump(mem);
    stages::humanoid_ext::dump(mem);
    stages::datamodel_ext::dump(mem);
    stages::sky::dump(mem);
    stages::character_ext::dump(mem);

    let elapsed = start.elapsed().as_millis();

    let offsets = G_DUMPER.offsets.lock().unwrap();
    writer::write_offsets(&offsets, elapsed);
}
