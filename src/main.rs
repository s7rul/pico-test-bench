use std::{collections::HashMap, fs, time::Duration, fmt::Display};

use object::{Object, ObjectSymbol};
use probe_rs::{flashing, Core, MemoryInterface, Permissions, Session};
use symex::{general_assembly::RunConfig, run_elf};

struct Measurement {
    name: String,
    hw: u64,
    symex: u64,
}

impl Display for Measurement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\t{}\t{}", self.name, self.hw, self.symex)
    }
}

fn main() {
    println!("Utility to measure HW cycles for the rp2040");

    let mut session = Session::auto_attach("rp2040", Permissions::default()).unwrap();

    println!("attached to rp2040 {:?}", session.architecture());

    println!("name\t\thw\tsymex");
    for to_test in fs::read_dir("test_binarys").unwrap() {
        let path = to_test.unwrap().path();
        let path_str = path.to_string_lossy().to_string();
        let name = path_str.split('/').last().unwrap();
        let hw_measurement = measure_hw(&path_str, &mut session);

        let symex_measurement = measure_symex(&path_str);

        let measurement = Measurement {
            name: name.to_owned(),
            hw: hw_measurement,
            symex: symex_measurement,
        };

        println!("{}", measurement);

    }
}

fn measure_symex(path: &str) -> u64 {
    let cfg = RunConfig {
        show_path_results: false,
        pc_hooks: vec![],
        register_read_hooks: vec![],
        register_write_hooks: vec![],
        memory_write_hooks: vec![],
        memory_read_hooks: vec![],
    };
    let results = run_elf::run_elf(path, "measure", cfg).unwrap();
    let mut max = 0;

    for result in results {
        max = max.max(result.max_cycles);
    }

    max as u64
}

fn measure_hw(path: &str, session: &mut Session) -> u64 {
    flashing::download_file(session, path, flashing::Format::Elf).unwrap();
    let mut core = session.core(0).unwrap();

    // Setup for measurement
    core.halt(Duration::from_millis(500)).unwrap();
    core.clear_all_hw_breakpoints().unwrap();

    // Start program
    core.reset().unwrap();

    // Wait until first measuring point
    core.wait_for_core_halted(Duration::from_millis(500))
        .unwrap();
    let start = core.read_word_32(0xe000e018).unwrap() & 0x00FFFFFF;

    // run until next measuring point
    core.run().unwrap();
    core.wait_for_core_halted(Duration::from_millis(500))
        .unwrap();

    let end = core.read_word_32(0xe000e018).unwrap() & 0x00FFFFFF;

    // calculate a measured time
    // compensate for bkpt discrepancy by subtracting 6 (determined by experimentation)
    let diff = start - end - 6;
    diff as u64
}
