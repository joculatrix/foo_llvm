use std::{error::Error, path::PathBuf};

use inkwell::{
    module::Module,
    targets::{
        CodeModel, FileType, InitializationConfig, RelocMode, Target,
        TargetMachine, TargetTriple
    },
    OptimizationLevel
};

pub fn init_target(triple: &Option<String>) -> Result<Target, Box<dyn Error>> {
    // initialize targets
    Target::initialize_all(&InitializationConfig::default());

    // set triple (e.g. x86_64-linux-gnu)
    let triple = if let Some(t) = triple {
        TargetTriple::create(t)
    } else {
        // detect default triple for the current machine
        TargetMachine::get_default_triple()
    };

    match Target::from_triple(&triple) {
        Ok(target) => Ok(target),
        Err(e) => Err(Box::new(e)),
    }
}

pub fn machine_from_target(target: &Target) -> Option<TargetMachine> {
    target.create_target_machine(
        &TargetMachine::get_default_triple(),
        "generic",
        "",
        OptimizationLevel::Default,
        RelocMode::PIC,
        CodeModel::Default,
    )
}

pub fn write_code_to_file(
    machine: &TargetMachine,
    module: &Module,
    path: &PathBuf,
    file_type: FileType
) -> Result<(), Box<dyn Error>> {
    Ok(machine.write_to_file(module, file_type, path)?)
}