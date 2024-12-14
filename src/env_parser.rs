use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Parse a `.env` file and return key-value pairs of environment variables.
pub fn parse_env_file(file_path: &PathBuf) -> Result<HashMap<String, String>> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read .env file at {}", file_path.display()))?;

    parse_env_str(&content)
}

/// Parse a `.env` format string and return key-value pairs.
///
/// Rules:
/// - Ignore empty lines.
/// - Ignore lines starting with `#` or `#!` (shebang).
/// - If a line contains a `#`, treat that and the rest of the line as a comment.
/// - Keys and values are trimmed.
/// - Invalid lines (no `=` or empty key/value) are ignored.
pub fn parse_env_str(content: &str) -> Result<HashMap<String, String>> {
    let mut env_vars = HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        // Ignore empty lines
        if line.is_empty() {
            continue;
        }

        // Ignore shebang or line that starts with '#'
        if line.starts_with("#!") || line.starts_with('#') {
            continue;
        }

        // Strip trailing comments
        let line = if let Some(idx) = line.find('#') {
            &line[..idx]
        } else {
            line
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some((key, value)) = parse_env_line(line) {
            env_vars.insert(key, value);
        }
    }

    Ok(env_vars)
}

/// Parse a single line of the form `KEY=VALUE`.
fn parse_env_line(line: &str) -> Option<(String, String)> {
    let mut split = line.splitn(2, '=');
    let key = split.next()?.trim();
    let val = split.next()?.trim();

    if key.is_empty() || val.is_empty() {
        return None;
    }

    Some((key.to_string(), val.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_env_line() {
        assert_eq!(
            parse_env_line("KEY=VALUE"),
            Some(("KEY".to_string(), "VALUE".to_string()))
        );
        assert_eq!(
            parse_env_line(" KEY = VALUE "),
            Some(("KEY".to_string(), "VALUE".to_string()))
        );
        assert_eq!(parse_env_line("EMPTY= "), None);
        assert_eq!(parse_env_line("NOEQUALS"), None);
        assert_eq!(parse_env_line("#COMMENT"), None);
    }

    #[test]
    fn test_parse_env_str_basic() -> Result<()> {
        let input = r#"
            KEY=VALUE
            FOO=BAR
        "#;

        let vars = parse_env_str(input)?;
        assert_eq!(vars.get("KEY"), Some(&"VALUE".to_string()));
        assert_eq!(vars.get("FOO"), Some(&"BAR".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_env_str_comments_and_spaces() -> Result<()> {
        let input = r#"
            # A comment
            # Another comment

            MYNAME=Patrick # inline comment

            # Another line
            SHELL=/bin/bash
        "#;

        let vars = parse_env_str(input)?;
        assert_eq!(vars.get("MYNAME"), Some(&"Patrick".to_string()));
        assert_eq!(vars.get("SHELL"), Some(&"/bin/bash".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_env_str_shebang_and_complex_comment_lines() -> Result<()> {
        let input = r#"
            #!/usr/bin/env bash # this line should be ignored if present
            # This is a comment that should be ignored

            MYNAME=Patrick # comments at the end of the line should also be okay
                    # and comments that don't start at the beginning of the line should also be okay
        "#;

        let vars = parse_env_str(input)?;
        assert_eq!(vars.get("MYNAME"), Some(&"Patrick".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_env_str_invalid_lines() -> Result<()> {
        let input = r#"
            KEY=VALUE
            INVALIDLINE
            ANOTHER=GOODVALUE
            NOTHING=
        "#;

        let vars = parse_env_str(input)?;
        assert_eq!(vars.get("KEY"), Some(&"VALUE".to_string()));
        assert_eq!(vars.get("ANOTHER"), Some(&"GOODVALUE".to_string()));
        // "INVALIDLINE" and "NOTHING=" should be ignored.
        assert!(!vars.contains_key("INVALIDLINE"));
        assert!(!vars.contains_key("NOTHING"));
        Ok(())
    }
}
