mod config;
mod input;
mod modules;
mod render;
mod style;

use std::io::{self, Read};
use std::process::ExitCode;

use config::Config;
use input::Input;

fn main() -> ExitCode {
    let mut raw = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut raw) {
        eprintln!("ccline: failed to read stdin: {e}");
        println!("ccline");
        return ExitCode::SUCCESS;
    }

    let input: Input = match serde_json::from_str(&raw) {
        Ok(input) => input,
        Err(e) => {
            eprintln!("ccline: failed to parse stdin JSON: {e}");
            println!("ccline");
            return ExitCode::SUCCESS;
        }
    };

    let config = Config::load();
    let line = render::render(&config.format, &input);
    println!("{line}");

    ExitCode::SUCCESS
}
