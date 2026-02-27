use clap::Parser;
use util::terminal::Styles;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let styles: &'static Styles = Box::leak(Box::new(Styles::detect_stderr()));
    let cli = attractor::cli::Cli::parse();

    let result = match cli.command {
        attractor::cli::Command::Run(args) => attractor::cli::run::run_command(args, styles).await,
        attractor::cli::Command::Validate(args) => {
            attractor::cli::validate::validate_command(&args, styles)
        }
        #[cfg(feature = "server")]
        attractor::cli::Command::Serve(args) => {
            attractor::cli::serve::serve_command(args, styles).await
        }
    };

    if let Err(e) = result {
        eprintln!(
            "{red}Error:{reset} {e:#}",
            red = styles.red, reset = styles.reset,
        );
        std::process::exit(1);
    }
}
