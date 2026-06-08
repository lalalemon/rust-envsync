mod sync;
mod validate;

use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

use sync::EnvSync;
use validate::{EnvValidator, ValidationRule};

#[derive(Parser)]
#[command(name = "envsync", version, about = "Sync .env files across environments with diffing and validation")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show the diff between two .env files
    Diff {
        /// Base .env file
        #[arg(short, long)]
        base: PathBuf,

        /// Target .env file to compare against
        #[arg(short, long)]
        target: PathBuf,

        /// Output format: text or json
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Merge two .env files (target values take precedence)
    Merge {
        /// Base .env file
        #[arg(short, long)]
        base: PathBuf,

        /// Target .env file to merge from
        #[arg(short, long)]
        target: PathBuf,

        /// Output file path (prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate a .env file against rules
    Validate {
        /// .env file to validate
        #[arg(short, long)]
        file: PathBuf,

        /// Use built-in common validation rules
        #[arg(long)]
        common: bool,

        /// Validate PORT is a valid port number
        #[arg(long)]
        check_ports: bool,

        /// Validate URLs start with http(s)://
        #[arg(long)]
        check_urls: bool,
    },

    /// List all variables in a .env file
    List {
        /// .env file to list
        #[arg(short, long)]
        file: PathBuf,

        /// Show line numbers
        #[arg(short, long)]
        line_numbers: bool,

        /// Sort by key name
        #[arg(short, long)]
        sort: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Diff { base, target, format } => {
            cmd_diff(&base, &target, &format);
        }
        Commands::Merge { base, target, output } => {
            cmd_merge(&base, &target, output.as_deref());
        }
        Commands::Validate {
            file,
            common,
            check_ports,
            check_urls,
        } => {
            cmd_validate(&file, common, check_ports, check_urls);
        }
        Commands::List {
            file,
            line_numbers,
            sort,
        } => {
            cmd_list(&file, line_numbers, sort);
        }
    }
}

fn cmd_diff(base: &PathBuf, target: &PathBuf, format: &str) {
    let base_env = match EnvSync::from_file(base) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            std::process::exit(1);
        }
    };
    let target_env = match EnvSync::from_file(target) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            std::process::exit(1);
        }
    };

    let diff = base_env.diff(&target_env);

    if format == "json" {
        print_diff_json(&diff);
        return;
    }

    println!(
        "{} {} vs {}",
        "Diff:".bold(),
        base.display(),
        target.display()
    );
    println!();

    if diff.added.is_empty() && diff.removed.is_empty() && diff.changed.is_empty() {
        println!("  {}", "No differences found.".green());
        return;
    }

    if !diff.added.is_empty() {
        println!("  {} ({}):", "Added".green().bold(), diff.added.len());
        for var in &diff.added {
            println!("    {} {}={}", "+".green(), var.key.cyan(), var.value);
        }
        println!();
    }

    if !diff.removed.is_empty() {
        println!("  {} ({}):", "Removed".red().bold(), diff.removed.len());
        for var in &diff.removed {
            println!("    {} {}={}", "-".red(), var.key.cyan(), var.value);
        }
        println!();
    }

    if !diff.changed.is_empty() {
        println!("  {} ({}):", "Changed".yellow().bold(), diff.changed.len());
        for var in &diff.changed {
            println!(
                "    {} {}: {} → {}",
                "~".yellow(),
                var.key.cyan(),
                var.old_value.dimmed(),
                var.new_value
            );
        }
        println!();
    }

    println!(
        "  {} {} variables unchanged",
        "=".dimmed(),
        diff.unchanged.len()
    );
}

fn print_diff_json(diff: &sync::EnvDiff) {
    println!("{{");
    println!("  \"added\": [");
    for (i, var) in diff.added.iter().enumerate() {
        let comma = if i < diff.added.len() - 1 { "," } else { "" };
        println!("    {{\"key\": \"{}\", \"value\": \"{}\"}}{}", var.key, var.value, comma);
    }
    println!("  ],");
    println!("  \"removed\": [");
    for (i, var) in diff.removed.iter().enumerate() {
        let comma = if i < diff.removed.len() - 1 { "," } else { "" };
        println!("    {{\"key\": \"{}\", \"value\": \"{}\"}}{}", var.key, var.value, comma);
    }
    println!("  ],");
    println!("  \"changed\": [");
    for (i, var) in diff.changed.iter().enumerate() {
        let comma = if i < diff.changed.len() - 1 { "," } else { "" };
        println!(
            "    {{\"key\": \"{}\", \"old_value\": \"{}\", \"new_value\": \"{}\"}}{}",
            var.key, var.old_value, var.new_value, comma
        );
    }
    println!("  ]");
    println!("}}");
}

fn cmd_merge(base: &PathBuf, target: &PathBuf, output: Option<&std::path::Path>) {
    let mut base_env = match EnvSync::from_file(base) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            std::process::exit(1);
        }
    };
    let target_env = match EnvSync::from_file(target) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            std::process::exit(1);
        }
    };

    let before_count = base_env.vars().len();
    base_env.merge(&target_env);
    let after_count = base_env.vars().len();

    let result = base_env.to_string_pretty();

    match output {
        Some(path) => {
            if let Err(e) = std::fs::write(path, &result) {
                eprintln!("{} Failed to write {}: {}", "error:".red().bold(), path.display(), e);
                std::process::exit(1);
            }
            println!(
                "{} Merged {} → {} ({} vars → {} vars)",
                "✓".green().bold(),
                base.display(),
                path.display(),
                before_count,
                after_count
            );
        }
        None => {
            println!("{}", result);
        }
    }
}

fn cmd_validate(file: &PathBuf, common: bool, check_ports: bool, check_urls: bool) {
    let env = match EnvSync::from_file(file) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            std::process::exit(1);
        }
    };

    let mut validator = EnvValidator::new();

    if common {
        validator = validator.with_common_rules();
    }

    if check_ports {
        // Find likely port variables and validate them
        for (key, var) in env.vars() {
            let key_upper = key.to_uppercase();
            if key_upper.contains("PORT") {
                validator = validator.rule(key, ValidationRule::Port);
            }
        }
    }

    if check_urls {
        for (key, _var) in env.vars() {
            let key_upper = key.to_uppercase();
            if key_upper.contains("URL") || key_upper.contains("URI") || key_upper.contains("ENDPOINT") {
                validator = validator.rule(key, ValidationRule::Url);
            }
        }
    }

    let report = validator.validate(env.vars());

    println!("{} {}", "Validating:".bold(), file.display());
    println!();

    if report.is_valid() {
        println!("  {} All validations passed!", "✓".green().bold());
    } else {
        println!(
            "  {} {} validation(s) failed:",
            "✗".red().bold(),
            report.errors().len()
        );
        for err in report.errors() {
            if let Some(msg) = &err.message {
                println!("    {} {}", "•".red(), msg);
            }
        }
        std::process::exit(1);
    }
}

fn cmd_list(file: &PathBuf, line_numbers: bool, sort: bool) {
    let env = match EnvSync::from_file(file) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            std::process::exit(1);
        }
    };

    let mut entries: Vec<_> = env.vars().values().collect();

    if sort {
        entries.sort_by(|a, b| a.key.cmp(&b.key));
    } else {
        entries.sort_by(|a, b| a.line_number.cmp(&b.line_number));
    }

    println!("{} ({} variables)\n", file.display(), entries.len());

    for var in &entries {
        if line_numbers {
            println!(
                "  {:>4}  {}={}",
                var.line_number.to_string().dimmed(),
                var.key.cyan(),
                var.value
            );
        } else {
            println!("  {}={}", var.key.cyan(), var.value);
        }
    }
}
