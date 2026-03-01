mod config;

use config::{load, ConfigOptions, EffectiveConfig};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug)]
struct StatusArgs {
    project_conf: String,
    format: OutputFormat,
}

fn main() -> std::io::Result<()> {
    match parse_args(std::env::args().skip(1).collect()) {
        Ok(Command::Status(args)) => run_status(args),
        Ok(Command::Help) => {
            print_help();
            Ok(())
        }
        Err(message) => {
            eprintln!("error: {message}\n");
            print_help();
            std::process::exit(2);
        }
    }
}

enum Command {
    Status(StatusArgs),
    Help,
}

fn parse_args(args: Vec<String>) -> Result<Command, String> {
    if args.is_empty() {
        return Ok(Command::Help);
    }

    if args[0] == "--help" || args[0] == "-h" {
        return Ok(Command::Help);
    }

    match args[0].as_str() {
        "status" => parse_status_args(&args[1..]).map(Command::Status),
        unknown => Err(format!("unknown subcommand: {unknown}")),
    }
}

fn parse_status_args(args: &[String]) -> Result<StatusArgs, String> {
    let mut project_conf = "project.conf".to_string();
    let mut format = OutputFormat::Text;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--project-conf" => {
                let Some(value) = args.get(i + 1) else {
                    return Err("missing value for --project-conf".to_string());
                };
                project_conf = value.clone();
                i += 2;
            }
            "--format" => {
                let Some(value) = args.get(i + 1) else {
                    return Err("missing value for --format".to_string());
                };
                format = parse_format(value)?;
                i += 2;
            }
            "--help" | "-h" => {
                print_status_help();
                std::process::exit(0);
            }
            unknown => {
                return Err(format!("unknown argument for status: {unknown}"));
            }
        }
    }

    Ok(StatusArgs {
        project_conf,
        format,
    })
}

fn parse_format(value: &str) -> Result<OutputFormat, String> {
    match value {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        _ => Err(format!("invalid --format value: {value} (expected: text or json)")),
    }
}

fn run_status(args: StatusArgs) -> std::io::Result<()> {
    let cfg = load(&ConfigOptions {
        project_conf_path: args.project_conf,
    })?;

    match args.format {
        OutputFormat::Text => {
            print_text_status(&cfg);
            Ok(())
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&cfg).map_err(std::io::Error::other)?);
            Ok(())
        }
    }
}

fn print_text_status(cfg: &EffectiveConfig) {
    println!("nocodo status");
    println!("project_name={}", cfg.project_name.as_deref().unwrap_or("<unset>"));
    println!("db_kind={}", cfg.db_kind);
    println!("database_url={}", cfg.database_url);
    println!("api_host={}", cfg.api_host);
    println!("api_port={}", cfg.api_port);
    println!("api_bind_addr={}", cfg.api_bind_addr());
}

fn print_help() {
    println!("nocodo-core <SUBCOMMAND> [OPTIONS]\n");
    println!("Subcommands:");
    println!("  status     Show effective runtime configuration");
    println!("\nRun `nocodo-core status --help` for status options.");
}

fn print_status_help() {
    println!("nocodo-core status [OPTIONS]\n");
    println!("Options:");
    println!("  --project-conf <PATH>   Path to project.conf (default: project.conf)");
    println!("  --format <text|json>    Output format (default: text)");
}
