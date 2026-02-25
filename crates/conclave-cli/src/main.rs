mod cmd;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "conclave",
    version = "0.1.0",
    about = "Intent-first, deterministic programming model"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Lower a `.conclave` source file to Plan IR JSON.
    Lower(cmd::lower::LowerArgs),
    /// Parse and canonicalize a Plan IR JSON file (also accepts `.conclave` source).
    Plan(cmd::plan::PlanArgs),
    /// Seal a program: pin capabilities, validate determinism, emit manifest.
    Seal(cmd::seal::SealArgs),
    /// Pack a sealed program into a runnable artifact.
    Pack(cmd::pack::PackArgs),
    /// Execute a sealed artifact.
    Run(cmd::run::RunArgs),
    /// Inspect a sealed artifact: print hashes, bindings, and policies.
    Inspect(cmd::inspect::InspectArgs),
    /// Install a capability binary into the content-addressed store.
    InstallCap(cmd::install_cap::InstallCapArgs),
    /// Manage the local module cache (publish, list, inspect).
    Module(cmd::module::ModuleArgs),
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Lower(args) => cmd::lower::run(args),
        Commands::Plan(args) => cmd::plan::run(args),
        Commands::Seal(args) => cmd::seal::run(args),
        Commands::Pack(args) => cmd::pack::run(args),
        Commands::Run(args) => cmd::run::run(args),
        Commands::Inspect(args) => cmd::inspect::run(args),
        Commands::InstallCap(args) => cmd::install_cap::run(args),
        Commands::Module(args) => cmd::module::run(args),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
