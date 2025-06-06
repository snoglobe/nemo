use anyhow::Result;
use clap::Parser as ClapParser;
use std::fs;
use std::path::PathBuf;

mod ast;
mod codegen;
mod parser;
mod type_checker;

#[derive(ClapParser, Debug)]
#[command(name = "lang-compiler")]
#[command(about = "Compiler for the spec.md language", long_about = None)]
struct Args {
    /// Input source file
    input: PathBuf,

    /// Output C file (default: input.c)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Print AST to stdout
    #[arg(long)]
    emit_ast: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Read source file
    let source = fs::read_to_string(&args.input)?;

    // Parse the source code
    let ast = parser::parse(&source)?;

    if args.emit_ast {
        println!("{:#?}", ast);
    }

    // Type check the AST
    let typed_ast = type_checker::check_program(ast)?;

    // Generate C code
    let c_code = codegen::generate(&typed_ast)?;

    // Determine output path
    let output_path = args.output.unwrap_or_else(|| {
        let mut path = args.input.clone();
        path.set_extension("c");
        path
    });

    // Write output
    fs::write(&output_path, c_code)?;

    println!("Successfully compiled {} to {}", args.input.display(), output_path.display());

    Ok(())
}