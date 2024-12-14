use anyhow::{Context, Result};
use clap::Parser;
use std::{collections::HashMap, env, path::PathBuf, process::Command};

mod env_parser;

static STRICT_WHITELIST: &[&str] = &["PATH", "HOME", "SHELL", "USER", "SHLVL", "LANG", "TERM"];

#[derive(Parser, Debug)]
#[command(
    name = "dotenv",
    author = "Patrick D'appollonio <hey@patrickdap.com>",
    about = "Dynamically inject just the environment variables you allow to the command you're about to execute."
)]
struct Cli {
    /// Specify the named environment file in ~/.dotenv/ (e.g. `example` for ~/.dotenv/example.env)
    #[arg(short, long)]
    environment: Option<String>,

    /// Strict mode: only environment variables from the .env file plus a minimal whitelist are kept
    #[arg(long)]
    strict: bool,

    /// The command and arguments to run (e.g. `python main.py`)
    #[arg(required = true)]
    command: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut strict = cli.strict;

    if cli.command.is_empty() {
        anyhow::bail!("No command provided.");
    }

    // Determine the environment file to use
    let env_file = match &cli.environment {
        Some(name) => get_named_env_file(name)?,
        None => {
            let current = env::current_dir().context("Could not get current directory")?;
            let file = current.join(".env");
            if file.exists() {
                Some(file)
            } else {
                None
            }
        }
    };

    // Load environment variables from the file if the file exists
    let env_vars_from_file = if let Some(file_path) = env_file {
        if file_path.exists() {
            env_parser::parse_env_file(&file_path).with_context(|| {
                format!("Could not parse environment file: {}", file_path.display(),)
            })?
        } else {
            HashMap::new()
        }
    } else {
        HashMap::new()
    };

    // Check if there is an env var for strict mode
    if !strict {
        if let Some(val) = env_vars_from_file.get("DOTENV_STRICT") {
            if is_truthy(val) {
                strict = true;
            }
        }
    }

    // Check if we finally have strict mode, if so, strip
    // all env vars except the whitelisted ones
    if strict {
        let mut new_env_vars: HashMap<String, String> = HashMap::new();
        for &var in STRICT_WHITELIST {
            if let Ok(val) = env::var(var) {
                new_env_vars.insert(var.to_string(), val);
            }
        }

        for (key, value) in env_vars_from_file {
            new_env_vars.insert(key, value);
        }

        clear_environment();

        for (key, value) in new_env_vars {
            env::set_var(key, value);
        }
    } else {
        // Strict mode is disabled, so we can inject all the
        // variables
        for (key, value) in env_vars_from_file {
            env::set_var(key, value);
        }
    }

    // Execute the program with the new variables
    let (program, args) = cli.command.split_first().context("No program specified")?;

    // Create the command and set the arguments apart so they outlive
    // the borrow checker
    let mut cmd = Command::new(program);
    cmd.args(args);

    // On Linux, set the Pdeathsig so the child receives SIGTERM if the parent dies
    #[cfg(target_os = "linux")]
    {
        use nix::sys::{prctl, signal};
        use nix::unistd;

        // Set PDEATHSIG to SIGTERM and SIGKILL
        prctl::set_pdeathsig(signal::Signal::SIGTERM).context("Failed to set PDEATHSIG")?;
        prctl::set_pdeathsig(signal::Signal::SIGKILL).context("Failed to set PDEATHSIG")?;

        // Double-check parent PID
        let ppid = unistd::getppid();
        if ppid == unistd::Pid::from_raw(1) {
            // The parent is init, meaning we won't get PDEATHSIG if the original parent is gone
            anyhow::bail!("Unable to operate on a program whose parent is init");
        }
    }

    // Grab the exit code from the executed program
    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute command: {}", program))?;
    std::process::exit(status.code().unwrap_or(1));
}

/// Clear all environment variables
fn clear_environment() {
    let keys: Vec<String> = env::vars().map(|(k, _)| k).collect();
    for key in keys {
        env::remove_var(key);
    }
}

/// Check if a value is considered truthy
fn is_truthy(value: &str) -> bool {
    matches!(
        value.to_lowercase().as_str(),
        "1" | "t" | "true" | "y" | "yes"
    )
}

fn get_named_env_file(name: &str) -> Result<Option<PathBuf>> {
    let home_dir = dirs::home_dir().context("Could not get home directory: the home directory is required to fetch specific environment files.")?;
    let dotenv_dir = home_dir.join(".dotenv");

    let file = dotenv_dir.join(format!("{}.env", name));
    if file.exists() {
        Ok(Some(file))
    } else {
        eprintln!(
            "Environment file does not exist in home directory settings folder: {}",
            file.display()
        );
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, path};
    use tempfile::NamedTempFile;

    #[test]
    fn test_is_truthy() {
        assert!(is_truthy("true"));
        assert!(is_truthy("True"));
        assert!(is_truthy("t"));
        assert!(is_truthy("T"));
        assert!(is_truthy("yes"));
        assert!(is_truthy("YES"));
        assert!(is_truthy("y"));
        assert!(is_truthy("Y"));
        assert!(is_truthy("1"));
        assert!(!is_truthy("false"));
        assert!(!is_truthy("no"));
        assert!(!is_truthy("0"));
        assert!(!is_truthy("random"));
    }

    #[test]
    fn test_dotenv_strict_sets_strict_mode() -> anyhow::Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, "DOTENV_STRICT=true")?;
        let location = path::absolute(file.path())?;
        let vars = env_parser::parse_env_file(&location)?;

        let mut strict = false;
        if let Some(val) = vars.get("DOTENV_STRICT") {
            if super::is_truthy(val) {
                strict = true;
            }
        }
        assert!(strict);

        let mut file2 = NamedTempFile::new()?;
        writeln!(file2, "DOTENV_STRICT=false")?;
        let location2 = path::absolute(file2.path())?;
        let vars2 = env_parser::parse_env_file(&location2)?;

        let mut strict2 = false;
        if let Some(val) = vars2.get("DOTENV_STRICT") {
            if super::is_truthy(val) {
                strict2 = true;
            }
        }
        assert!(!strict2);
        Ok(())
    }

    #[test]
    fn test_clear_environment() {
        env::set_var("TESTVAR", "VALUE");
        clear_environment();
        assert!(env::var("TESTVAR").is_err());
    }
}
