use std::path::PathBuf;

use chumsky::error::{Rich, RichReason};
use codesnake::{Block, CodeWidth, Label, LineIndex};
use yansi::Paint;

/// Take the errors output by the Chumsky parser and print them.
pub fn print_syntax_errors(errs: Vec<Rich<char>>, path: &PathBuf, src: &str) {
    let idx = LineIndex::new(&src);
    let errs = build_syntax_errors(errs, path, &idx);
    errs.iter().for_each(|err| err.print());
}

fn build_syntax_errors<'src>(
    errs: Vec<Rich<char>>,
    path: &PathBuf,
    idx: &'src LineIndex<'src>
) -> Vec<CompilerErr<'src>> {
    let mut res = vec![];

    for err in errs {
        let reason = err.reason();
        match reason {
            RichReason::ExpectedFound { expected, found } => {
                let msg = format!(
                    "[{:#?}]: invalid syntax, expected {}",
                    path.file_name().unwrap(),
                    expected.iter()
                        .fold(String::new(), |mut acc, e| {
                            acc.push_str(&e.to_string());
                            acc
                        })
                );
                let label = if let Some(token) = found {
                    Label::new(err.span().into_range())
                        .with_text(format!("found {}", token.into_inner()))
                        .with_style(|s| s.red().to_string())
                } else {
                    Label::new(err.span().into_range())
                        .with_style(|s| s.red().to_string())
                };

                let block = Block::new(&idx, [label]).unwrap();
                let block = block.map_code(|c| CodeWidth::new(c, c.len())); 

                res.push(CompilerErr { msg, block });
            }
            RichReason::Custom(msg) => {
                let msg = format!(
                    "[{:#?}]: {}",
                    path.file_name().unwrap(),
                    msg
                );
                let label = Label::new(err.span().into_range())
                    .with_text("here".to_owned())
                    .with_style(|s| s.red().to_string());
                let block = Block::new(&idx, [label]).unwrap();
                let block = block.map_code(|c| CodeWidth::new(c, c.len()));

                res.push(CompilerErr { msg, block });
            }
            RichReason::Many(_) => todo!(),
        }
    }
    res
}

struct CompilerErr<'a> {
    msg: String,
    block: Block<CodeWidth<&'a str>, String>,
}

impl<'a> CompilerErr<'a> {
    pub fn print(&self) {
        eprintln!("{}{}", self.block.prologue(), self.msg);
        eprint!("{}", self.block);
        eprintln!("{}", self.block.epilogue());
    }
}