use anyhow::{Context, Result};
use clap::Parser;
use std::{collections::HashMap, env, path::PathBuf, process::Command};

mod env_parser;

static STRICT_WHITELIST: &[&str] = &[
    "PATH", "HOME", "SHELL", "USER", "SHLVL", "LANG", "TERM", "LOGNAME", "PWD", "OLDPWD", "EDITOR",
    "VISUAL", "DISPLAY", "HOSTNAME",
];

#[derive(Parser, Debug)]
#[command(
    name = "dotenv",
    author = "Patrick D'appollonio <hey@patrickdap.com>",
    about = "Dynamically inject environment variables from .env files into the command you're about to execute. By default reads .env from the current directory unless --file is specified."
)]
struct Cli {
    /// Specify a custom environment file path (defaults to .env in current directory)
    #[arg(short = 'f', long = "file")]
    envfile: Option<PathBuf>,

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

    // Load global environment file first (if provided)
    let mut env_vars_from_file = if let Some(name) = &cli.environment {
        if let Some(global_file) = get_named_env_file(name)? {
            env_parser::parse_env_file(&global_file).with_context(|| {
                format!(
                    "Could not parse global environment file: {}",
                    global_file.display()
                )
            })?
        } else {
            HashMap::new()
        }
    } else {
        HashMap::new()
    };

    // Load local environment file second (overwrites global)
    let local_env_vars = if let Some(custom_path) = &cli.envfile {
        // Custom file path specified - it must exist
        if !custom_path.exists() {
            anyhow::bail!(
                "custom environment file does not exist: {}",
                custom_path.display()
            );
        }
        env_parser::parse_env_file(custom_path).with_context(|| {
            format!(
                "Could not parse custom environment file: {}",
                custom_path.display()
            )
        })?
    } else {
        // Default to local .env file if it exists
        let current = env::current_dir().context("Could not get current directory")?;
        let file = current.join(".env");
        if file.exists() {
            env_parser::parse_env_file(&file).with_context(|| {
                format!("Could not parse local environment file: {}", file.display())
            })?
        } else {
            HashMap::new()
        }
    };

    // Merge local environment variables into global ones (local overwrites global)
    for (key, value) in local_env_vars {
        env_vars_from_file.insert(key, value);
    }

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
        use std::io::{Error, ErrorKind};
        use std::os::unix::process::CommandExt;

        unsafe {
            cmd.pre_exec(|| {
                // Set the parent-death signal to SIGTERM
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM, 0, 0, 0) != 0 {
                    return Err(Error::last_os_error());
                }

                // Set the parent-death signal to SIGKILL
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0) != 0 {
                    return Err(Error::last_os_error());
                }

                // Double-check parent PID
                let ppid = libc::getppid();
                if ppid == 1 {
                    // The parent is init, meaning we won't get PDEATHSIG if the original parent is gone
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Unable to operate on a program whose parent is init",
                    ));
                }

                Ok(())
            });
        }
    }

    // Grab the exit code from the executed program
    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute command: {program}"))?;
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

    let file = dotenv_dir.join(format!("{name}.env"));
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
    use serial_test::serial;
    use std::{io::Write, path};
    use tempfile::NamedTempFile;

    #[test]
    #[serial]
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
    #[serial]
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
    #[serial]
    fn test_clear_environment() {
        env::set_var("TESTVAR", "VALUE");
        clear_environment();
        assert!(env::var("TESTVAR").is_err());
    }

    #[test]
    #[serial]
    fn test_strict_mode_removes_unlisted_vars() -> anyhow::Result<()> {
        // Set some environment variables that should NOT persist in strict mode
        env::set_var("UNSAFE_VAR", "123");
        env::set_var("PATH", "/usr/bin"); // PATH is whitelisted and should remain

        let mut file = NamedTempFile::new()?;
        writeln!(file, "CUSTOM_VAR=Hello")?;
        let location = path::absolute(file.path())?;

        // Simulate CLI arguments: --strict and a dummy command (e.g. "echo")
        let cli_args = vec![
            "dotenv",
            "--strict",
            "--environment",
            location.to_str().unwrap(),
            "echo",
            "test",
        ];
        let cli = Cli::parse_from(cli_args);
        assert!(cli.strict);

        // Clear environment in the main function and re-set it based on strict mode
        clear_environment();
        env::set_var("UNSAFE_VAR", "123");
        env::set_var("PATH", "/usr/bin");

        let env_vars_from_file = env_parser::parse_env_file(&location)?;
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
        for (key, value) in new_env_vars.clone() {
            env::set_var(key, value);
        }

        // Check environment after strict mode application
        assert!(env::var("UNSAFE_VAR").is_err());
        assert_eq!(env::var("CUSTOM_VAR").unwrap(), "Hello");
        assert!(env::var("PATH").is_ok());

        Ok(())
    }

    #[test]
    #[serial]
    fn test_non_strict_mode_keeps_existing_vars() -> anyhow::Result<()> {
        // Simulate existing environment variable
        env::set_var("EXISTING_VAR", "EXISTING_VALUE");

        let mut file = NamedTempFile::new()?;
        writeln!(file, "NEW_VAR=NEW_VALUE")?;
        let location = path::absolute(file.path())?;

        // Run without --strict
        let cli_args = vec![
            "dotenv",
            "--environment",
            location.to_str().unwrap(),
            "echo",
            "test",
        ];
        let cli = Cli::parse_from(cli_args);
        assert!(!cli.strict);

        let env_vars_from_file = env_parser::parse_env_file(&location)?;

        for (key, value) in env_vars_from_file {
            env::set_var(key, value);
        }

        // Check that both the existing var and new var are present
        assert_eq!(env::var("EXISTING_VAR").unwrap(), "EXISTING_VALUE");
        assert_eq!(env::var("NEW_VAR").unwrap(), "NEW_VALUE");
        Ok(())
    }

    #[test]
    #[serial]
    fn test_missing_environment_file() -> anyhow::Result<()> {
        let non_existent = PathBuf::from("this_file_does_not_exist.env");

        // Attempt to parse a non-existent file
        let vars = env_parser::parse_env_file(&non_existent);
        // Since the function tries to read a non-existent file, it should error out
        // but you might handle this differently in your code. If you return Ok with empty,
        // adjust this test accordingly.
        assert!(vars.is_err());

        Ok(())
    }

    #[test]
    #[serial]
    fn test_env_file_strict_mode_applied() -> anyhow::Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, "DOTENV_STRICT=true")?;
        writeln!(file, "MYVAR=SHOULD_EXIST")?;
        let location = path::absolute(file.path())?;

        let cli_args = vec![
            "dotenv",
            "--environment",
            location.to_str().unwrap(),
            "echo",
            "test",
        ];
        let cli = Cli::parse_from(cli_args);

        let mut strict = cli.strict;
        let env_vars = env_parser::parse_env_file(&location)?;
        if !strict {
            if let Some(val) = env_vars.get("DOTENV_STRICT") {
                if is_truthy(val) {
                    strict = true;
                }
            }
        }
        assert!(strict);

        Ok(())
    }

    #[test]
    #[serial]
    fn test_env_overrides_system_in_non_strict_mode() -> anyhow::Result<()> {
        // Set a system var
        env::set_var("FOO", "SYSTEM_VALUE");

        let mut file = NamedTempFile::new()?;
        writeln!(file, "FOO=FILE_VALUE")?;
        let location = path::absolute(file.path())?;

        // Non-strict mode
        let env_vars = env_parser::parse_env_file(&location)?;
        for (key, value) in env_vars {
            env::set_var(key, value);
        }

        // The environment var should now be overridden
        assert_eq!(env::var("FOO").unwrap(), "FILE_VALUE");
        Ok(())
    }
}
