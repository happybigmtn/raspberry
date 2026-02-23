use clap::Parser;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let cli = attractor::cli::Cli::parse();

    let result = match cli.command {
        attractor::cli::Command::Run(args) => attractor::cli::run::run_command(args).await,
        attractor::cli::Command::Validate(args) => attractor::cli::validate::validate_command(&args),
    };

    if let Err(e) = result {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}
