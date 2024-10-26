use std::{error::Error, fs::File, path::PathBuf, process};

use chumsky::Parser;
use clap::ValueEnum;
use inkwell::targets::FileType;
use llvm::{print_module, LlvmGenerator};
use parse::parser;

mod error;
mod llvm;
mod parse;

/// Example LLVM-based compiler for a simple language
#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Source file to compile
    src: PathBuf,
    /// Path of file to output
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// The type of output to produce
    #[arg(short, long, value_enum, default_value = "llvm-ir")]
    produce: OutputType,
    /// Target triple of the intended target machine to build for,
    /// in form <arch><sub_arch>-<vendor>-<sys>-<env>, e.g. x86_64-linux-gnu
    #[arg(short, long)]
    target: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OutputType {
    /// Output object file (.o)
    Object,
    /// Output assembly code (.s)
    Assembly,
    /// Output LLVM bitcode (.bc)
    Bitcode,
    /// Output LLVM IR (to stderr; specify an output path to write to a file, 
    /// typically .ll)
    LlvmIR,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = {
        use clap::Parser;
        Args::parse()
    };

    let Ok(src) = std::fs::read_to_string(args.src.clone()) else {
        return Err("failed to open file".into());
    };

    let ast = parser()
        .parse(&src)
        .into_result()
        .unwrap_or_else(|errs| {
            error::print_syntax_errors(errs, &args.src, &src);
            std::process::exit(1);
        });

    let target = match llvm::init_target(&args.target) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    let context = inkwell::context::Context::create();
    let module = context.create_module("foo");
    let builder = context.create_builder();

    // best practice: optionally set the data layout for the module based
    // on target machine
    if let Some(machine) = llvm::machine_from_target(&target) {
        module.set_data_layout(&machine.get_target_data().get_data_layout());
    }

    match LlvmGenerator::generate(&ast, &context, &module, &builder) {
        Ok(_) => {
            match args.produce {
                OutputType::Object => {
                    let path = get_output_path(args.output, "foo.o")?;
                    // use scope to drop file after ensuring it exists
                    { let _ = open_file(&path)?; }
                    let Some(machine) = llvm::machine_from_target(&target) else {
                        return Err("failed to build target machine".into());
                    };
                    llvm::write_code_to_file(
                        &machine,
                        &module,
                        &path,
                        FileType::Object
                    )?;
                }
                OutputType::Assembly => {
                    let path = get_output_path(args.output, "foo.s")?;
                    // use scope to drop file after ensuring it exists
                    { let _ = open_file(&path)?; }
                    let Some(machine) = llvm::machine_from_target(&target) else {
                        return Err("failed to build target machine".into());
                    };
                    llvm::write_code_to_file(
                        &machine,
                        &module,
                        &path,
                        FileType::Assembly
                    )?;
                }
                OutputType::Bitcode => {
                    let path = get_output_path(args.output, "foo.bc")?;
                    // use scope to drop file after ensuring it exists
                    { let _ = open_file(&path)?; }
                    module.write_bitcode_to_path(&path);
                }
                OutputType::LlvmIR => {
                    if let Some(path) = args.output {
                        let mut file = open_file(&path)?;
                        llvm::write_module_to_file(&module, &mut file)?;
                    } else {
                        print_module(&module);
                    }
                }
            }
        }
        Err(e) => eprintln!("{}", e),
    }

    Ok(())
}

fn open_file(path: &PathBuf) -> Result<File, Box<dyn Error>> {
    if path.exists() && !path.is_file() {
        return Err("output path isn't a file name".into());
    }

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }

    Ok(File::create(&path)?)
}

fn get_output_path(
    path: Option<PathBuf>,
    default: &str
) -> Result<PathBuf, Box<dyn Error>> {
    if let Some(path) = path {
        if path.is_file() || !path.exists() {
            Ok(path)
        } else {
            Err(format!("{:#?} exists and isn't a file", path).into())
        }
    } else {
        let mut path = PathBuf::new();
        path.set_file_name(default);
        Ok(path)
    }
}