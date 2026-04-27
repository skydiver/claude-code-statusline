mod config;
mod dump;
mod input;
mod modules;
mod render;
mod style;

use std::io::{self, IsTerminal, Read};
use std::process::ExitCode;

use config::Config;
use input::Input;

fn main() -> ExitCode {
    let stdin = io::stdin();

    // Running `ccline` directly in a terminal means there's no piped Claude
    // Code payload — `read_to_string` would block forever waiting for EOF.
    // Print a usage banner and exit cleanly instead.
    if stdin.is_terminal() {
        print_usage();
        return ExitCode::SUCCESS;
    }

    let mut raw = String::new();
    if let Err(e) = stdin.lock().read_to_string(&mut raw) {
        eprintln!("ccline: failed to read stdin: {e}");
        println!("ccline");
        return ExitCode::SUCCESS;
    }

    // Parse to a generic Value first so the optional dump file is a faithful
    // copy of what Claude Code sent — including any fields the typed `Input`
    // struct does not yet model. Then convert to the typed view for rendering.
    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("ccline: failed to parse stdin JSON: {e}");
            println!("ccline");
            return ExitCode::SUCCESS;
        }
    };

    dump::try_dump(&value);

    let input: Input = match serde_json::from_value(value) {
        Ok(input) => input,
        Err(e) => {
            eprintln!("ccline: failed to interpret stdin JSON: {e}");
            println!("ccline");
            return ExitCode::SUCCESS;
        }
    };

    let config = Config::load();
    let line = render::render(&config.format, &input);
    println!("{line}");

    ExitCode::SUCCESS
}

fn print_usage() {
    let version = env!("CARGO_PKG_VERSION");
    println!("ccline {version} — Claude Code statusline renderer");
    println!();
    println!("ccline reads the Claude Code session JSON from stdin and prints a");
    println!("formatted statusline to stdout. It is meant to be wired into Claude");
    println!("Code's settings file, not invoked directly.");
    println!();
    println!("Wire-up (~/.claude/settings.json):");
    println!("  {{");
    println!("    \"statusLine\": {{");
    println!("      \"type\": \"command\",");
    println!("      \"command\": \"/absolute/path/to/ccline\"");
    println!("    }}");
    println!("  }}");
    println!();
    println!("Config file (optional, TOML):");
    println!("  $XDG_CONFIG_HOME/claude-code-statusline/config.toml");
    println!("  ~/.config/claude-code-statusline/config.toml");
    println!();
    println!("Local test with the bundled fixture:");
    println!("  cat tests/fixtures/sample_input.json | ccline");
}
