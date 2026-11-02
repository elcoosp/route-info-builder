//! Binary entry point for the `route-info-builder` CLI.

fn main() {
    std::process::exit(route_info_builder::cli::run(std::env::args_os()));
}
