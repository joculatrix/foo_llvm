use std::{error::Error, path::PathBuf, process::Command};

use crate::Linker;

/// List of C compilers/linkers to attempt for linking to an executable.
/// 
/// * `gcc` - GNU compiler collection
/// * `ld` - GNU linker; unsure if this would exist in absence of gcc, but can't
///   hurt to check
/// * `lld` - LLVM linker
/// * `clang` - LLVM's C compiler frontend
/// * `link` - command for MSVC's linker
static LINKERS: [&str; 5] = ["clang", "gcc", "link", "ld", "lld"];

impl Linker {
    fn to_string(&self) -> &str {
        match self {
            Linker::Clang => LINKERS[0],
            Linker::Gcc => LINKERS[1],
            Linker::Link => LINKERS[2],
            Linker::Ld => LINKERS[3],
            Linker::Lld => LINKERS[4],
        }
    }
}
/// Attempts to use a specified linker, or any known linkers if none was
/// specified, to produce an executable from a given object or assembly file.
/// 
/// * `object` - the path to the object or assembly file produced by the compiler
/// * `linker` - the linker, if any, specified by the user via CLI args
pub fn try_to_bin(
    object: &PathBuf,
    out: &PathBuf,
    linker: Option<Linker>
) -> Result<(), Box<dyn Error>> {
    let out = out.to_str().unwrap().trim();
    // if the user specified a linker, the program should halt if that linker
    // doesn't work
    if let Some(linker) = linker {
        let args = if linker == Linker::Link {
            [object.to_str().unwrap(), &format!("/OUT:{}", out)]
        } else {
            [object.to_str().unwrap(), &format!("-o{}", out)]
        };

        let res = Command::new(linker.to_string())
            .args(args)
            .spawn();
        match res {
            Ok(mut cmd) => {
                cmd.wait()?;
                // clean up intermediary object file
                std::fs::remove_file(object)?;
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    return Err(
                        format!(
                            "command `{}` couldn't be found",
                            linker.to_string())
                        .into()
                    );
                }
                _ => {
                    return Err("unknown error occurred calling linker".into());
                }
            }
        }
        Ok(())
    // if the user didn't specify a linker, the program should try to find
    // any it knows about
    } else {
        for linker in LINKERS {
            let args = if linker == "link" {
                [object.to_str().unwrap(), &format!("/OUT:{}", out)]
            } else {
                [object.to_str().unwrap(), &format!("-o{}", out)]
            };

            let res = Command::new(linker).args(args).spawn();
            if let Ok(mut cmd) = res {
                cmd.wait()?;
                // clean up intermediary object file
                std::fs::remove_file(object)?;
                return Ok(());
            }
        }
        Err("no known linkers were found".into())
    }
}