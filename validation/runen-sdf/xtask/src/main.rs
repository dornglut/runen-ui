mod command;
mod fs_walk;
mod links;
mod policy;

fn main() {
    let result = match std::env::args().nth(1).as_deref() {
        Some("validate") => validate(),
        _ => Err("usage: cargo xtask validate".to_owned()),
    };

    if let Err(error) = result {
        eprintln!("validation failed: {error}");
        std::process::exit(1);
    }
}

fn validate() -> Result<(), String> {
    policy::validate_repository()?;
    links::validate_markdown_links()?;
    command::run_validation_commands()?;
    command::prove_clean_repository_state()
}
