This is a small (partly educational for myself, partly a potential example for
anyone else picking up these tools) project derived from the `Chumsky` parser
combinator crate's [tutorial](https://github.com/zesterer/chumsky/blob/main/tutorial.md).
Rather than the tutorial's in-Rust interpreter-style implementation, I've
modified the code to take the AST from the parser and transfer it to an LLVM
backend using [`Inkwell`](https://github.com/TheDan64/inkwell) (which provides a
Rust wrapper around LLVM's API).

## Running the project

To get an executable:


```
foo_llvm test.foo
```

You can also set the `-p` flag to `llvm-ir`, `assembly`, `bitcode`, or `object`
to get other types of output. `llvm-ir`, by default, is written to stderr
unless an output file is specified.

Producing an executable requires a C compiler or linker: the options the
program recognizes are `clang`, `gcc`, `ld`, `lld` (the LLVM linker), or
`link` (the MSVC linker) -- if none are specified, it will try each.

> Note: Some linkers (specifically the MSVC linker) are currently untested.

## Building from source

To build this project from source, you must have `llvm-config` and version 18
of LLVM installed. Ideally, `llvm-sys`'s build script should automatically be
able to locate your LLVM installation.

The `libffi` crate is also listed as a dependency. By default, this builds the
libffi library for interfacing with C by itself in order to include it. Per the
crate's documentation, if you have libffi installed and would prefer it use that
rather than building a new copy, you can change `Cargo.toml` to specify the
`system` feature:

```
[dependencies]
...
libffi = { version = "3.2.0", features = ["system"] }
```

Aside from issues you may (but hopefuly don't) have with linking LLVM or libffi,
the project should run with a simple `cargo run -- test.foo` without any other
trouble. I wrote it using Rust `1.82.0`.
