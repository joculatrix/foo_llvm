use std::error::Error;
use std::fs::File;
use std::io::Write;

use inkwell::module::Module;

mod ir;
mod target;

pub use ir::LlvmGenerator;
pub use target::init_target;
pub use target::machine_from_target;
pub use target::write_code_to_file;

/// Prints an LLVM module's contents to stderr.
pub fn print_module(module: &Module) {
    module.print_to_stderr();
}

pub fn write_module_to_file(
    module: &Module,
    file: &mut File
) -> Result<(), Box<dyn Error>> {
    let module = module.to_string();
    Ok(file.write_all(module.as_bytes())?)
}