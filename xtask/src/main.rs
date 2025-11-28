use std::{env, process};

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let task = env::args().nth(1);
    match task.as_deref() {
        Some("-h" | "--help") => print_help(),
        Some(task) => return Err(format!("unknown task: {task}")),
        None => print_help(),
    }
    Ok(())
}

fn print_help() {
    eprintln!(
        "Usage: cargo xtask <TASK>

Tasks:
    (no tasks defined yet)
"
    );
}
