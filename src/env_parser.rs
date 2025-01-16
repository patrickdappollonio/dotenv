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

    let val = strip_quotes(val).trim();

    Some((key.to_string(), val.to_string()))
}

/// Strip leading and trailing quotes from a string.
fn strip_quotes(s: &str) -> &str {
    let trimmed = s.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        let first = bytes[0];
        let last = bytes[trimmed.len() - 1];

        // Check if the value is wrapped in matching single or double quotes
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &trimmed[1..trimmed.len() - 1];
        }
    }

    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
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

    #[test]
    #[serial]
    fn test_parse_env_str_quoted_values() -> Result<()> {
        let input = r#"
        KEY="some value with spaces"
        ANOTHER='single quoted value'
        MIXED="leading and trailing spaces  "
    "#;

        let vars = parse_env_str(input)?;
        assert_eq!(vars.get("KEY"), Some(&"some value with spaces".to_string()));
        assert_eq!(
            vars.get("ANOTHER"),
            Some(&"single quoted value".to_string())
        );
        assert_eq!(
            vars.get("MIXED"),
            Some(&"leading and trailing spaces".to_string())
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn test_parse_env_str_values_with_equals() -> Result<()> {
        let input = r#"
        KEY=VALUE=WITH=EQUALS
        DATABASE_URL=mysql://user:pass@localhost/dbname
    "#;

        let vars = parse_env_str(input)?;
        assert_eq!(vars.get("KEY"), Some(&"VALUE=WITH=EQUALS".to_string()));
        assert_eq!(
            vars.get("DATABASE_URL"),
            Some(&"mysql://user:pass@localhost/dbname".to_string())
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn test_parse_env_str_whitespace_around_equals() -> Result<()> {
        let input = r#"
        KEY    =     VALUE
        TRIM   =    TRIMMED
    "#;

        let vars = parse_env_str(input)?;
        assert_eq!(vars.get("KEY"), Some(&"VALUE".to_string()));
        assert_eq!(vars.get("TRIM"), Some(&"TRIMMED".to_string()));
        Ok(())
    }

    #[test]
    #[serial]
    fn test_parse_env_str_empty_key_or_value() -> Result<()> {
        let input = r#"
        =VALUE
        KEY=
        =
        JUSTEMPTY
    "#;

        let vars = parse_env_str(input)?;
        // None of these should be included
        assert!(!vars.contains_key(""));
        assert!(!vars.contains_key("KEY"));
        assert!(!vars.contains_key("JUSTEMPTY"));
        Ok(())
    }

    #[test]
    #[serial]
    fn test_parse_env_str_complex_comments() -> Result<()> {
        let input = r#"
        # Initial comment
        KEY=VALUE  # inline comment
        # Another comment line
        ANOTHER=VAL     # comment after value
        # Yet another comment
    "#;

        let vars = parse_env_str(input)?;
        assert_eq!(vars.get("KEY"), Some(&"VALUE".to_string()));
        assert_eq!(vars.get("ANOTHER"), Some(&"VAL".to_string()));
        Ok(())
    }
}
