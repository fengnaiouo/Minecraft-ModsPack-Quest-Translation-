mod cli;
mod commands;
mod translation;

fn main() {
    if let Err(error) = commands::real_main() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}
