#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args = manifestador_nfe_lib::cli::args::CliArgs::parse();
    if manifestador_nfe_lib::cli::runner::should_run_without_tauri(&args) {
        if let Err(err) = manifestador_nfe_lib::cli::runner::run(args) {
            eprintln!("{err}");
            std::process::exit(1);
        }
        return;
    }
    #[cfg(feature = "desktop")]
    manifestador_nfe_lib::run();

    #[cfg(not(feature = "desktop"))]
    {
        eprintln!(
            "O modo desktop exige a feature Cargo 'desktop'. Use --headless, --cli ou --worker."
        );
        std::process::exit(2);
    }
}
