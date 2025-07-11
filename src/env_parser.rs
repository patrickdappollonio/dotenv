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
/// - Supports multiline quoted values and escape sequences.
/// - Supports line continuation with backslash at end of line.
pub fn parse_env_str(content: &str) -> Result<HashMap<String, String>> {
    let mut env_vars = HashMap::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Ignore empty lines
        if line.is_empty() {
            i += 1;
            continue;
        }

        // Ignore shebang or line that starts with '#'
        if line.starts_with("#!") || line.starts_with('#') {
            i += 1;
            continue;
        }

        // Strip trailing comments (but be careful with quotes)
        let line = strip_inline_comment(line);
        let line = line.trim();

        if line.is_empty() {
            i += 1;
            continue;
        }

        // Create a modified lines slice with the comment-stripped first line
        let mut modified_lines = vec![line];
        modified_lines.extend_from_slice(&lines[i + 1..]);

        // Try to parse as key=value, potentially multiline
        if let Some((key, value, lines_consumed)) = parse_env_entry(&modified_lines) {
            env_vars.insert(key, value);
            i += lines_consumed;
        } else {
            i += 1;
        }
    }

    Ok(env_vars)
}

/// Strip inline comments, being careful not to strip comments inside quoted strings
fn strip_inline_comment(line: &str) -> &str {
    let mut in_quotes = false;
    let mut quote_char = '"';
    let mut escaped = false;

    for (i, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if in_quotes => escaped = true,
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = ch;
            }
            ch if in_quotes && ch == quote_char => in_quotes = false,
            '#' if !in_quotes => return &line[..i].trim_end(),
            _ => {}
        }
    }

    line
}

/// Parse a single line of the form `KEY=VALUE`.
fn parse_env_line(line: &str) -> Option<(String, String)> {
    let mut split = line.splitn(2, '=');
    let key = split.next()?.trim();
    let val = split.next()?.trim();

    if key.is_empty() || val.is_empty() {
        return None;
    }

    // Check if value is quoted, if so process escape sequences
    let processed_val = if (val.starts_with('"') && val.ends_with('"'))
        || (val.starts_with('\'') && val.ends_with('\''))
    {
        let stripped = strip_quotes(val).trim();
        process_escape_sequences(stripped)
    } else {
        val.to_string()
    };

    Some((key.to_string(), processed_val))
}

/// Parse a potentially multiline environment entry
fn parse_env_entry(lines: &[&str]) -> Option<(String, String, usize)> {
    if lines.is_empty() {
        return None;
    }

    let first_line = lines[0].trim();
    let mut split = first_line.splitn(2, '=');
    let key = split.next()?.trim();
    let initial_value = split.next()?.trim();

    if key.is_empty() {
        return None;
    }

    // Handle different value types
    if initial_value.is_empty() {
        return None;
    }

    // Check if this is a quoted multiline value (unclosed quote)
    if (initial_value.starts_with('"') && !ends_with_unescaped_quote(initial_value, '"'))
        || (initial_value.starts_with('\'') && !ends_with_unescaped_quote(initial_value, '\''))
    {
        // Multiline quoted value
        let quote_char = initial_value.chars().next().unwrap();
        let mut value = String::from(&initial_value[1..]); // Remove opening quote
        let mut lines_consumed = 1;

        // Continue reading lines until we find the closing quote
        for (idx, &line) in lines[1..].iter().enumerate() {
            lines_consumed += 1;

            if let Some(end_pos) = find_unescaped_quote(line, quote_char) {
                // Found closing quote
                value.push('\n');
                value.push_str(&line[..end_pos]);
                break;
            } else {
                // Continue multiline
                value.push('\n');
                value.push_str(line);
            }

            // Safety check: don't consume too many lines
            if idx > 100 {
                return None;
            }
        }

        let processed_value = process_escape_sequences(&value);
        return Some((key.to_string(), processed_value, lines_consumed));
    }

    // Check if this is a line continuation (ends with \)
    if initial_value.ends_with('\\') && !initial_value.ends_with("\\\\") {
        let mut value = String::from(&initial_value[..initial_value.len() - 1]);
        let mut lines_consumed = 1;

        // Continue reading lines until we find one that doesn't end with \
        for &line in lines[1..].iter() {
            lines_consumed += 1;
            let trimmed = line.trim();

            if trimmed.ends_with('\\') && !trimmed.ends_with("\\\\") {
                value.push_str(&trimmed[..trimmed.len() - 1]);
            } else {
                value.push_str(trimmed);
                break;
            }

            // Safety check
            if lines_consumed > 100 {
                break;
            }
        }

        let processed_value = process_escape_sequences(&value);
        return Some((key.to_string(), processed_value, lines_consumed));
    }

    // Regular single-line value - use original logic
    if let Some((parsed_key, parsed_value)) = parse_env_line(first_line) {
        Some((parsed_key, parsed_value, 1))
    } else {
        None
    }
}

/// Find unescaped quote character in a string
fn find_unescaped_quote(s: &str, quote_char: char) -> Option<usize> {
    let mut escaped = false;

    for (i, ch) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
        } else if ch == quote_char {
            return Some(i);
        }
    }

    None
}

/// Process escape sequences in a string
fn process_escape_sequences(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next_ch) = chars.peek() {
                match next_ch {
                    'n' => {
                        result.push('\n');
                        chars.next();
                    }
                    't' => {
                        result.push('\t');
                        chars.next();
                    }
                    'r' => {
                        result.push('\r');
                        chars.next();
                    }
                    '\\' => {
                        result.push('\\');
                        chars.next();
                    }
                    '"' => {
                        result.push('"');
                        chars.next();
                    }
                    '\'' => {
                        result.push('\'');
                        chars.next();
                    }
                    _ => {
                        result.push(ch);
                    }
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    result
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

/// Check if a string ends with an unescaped quote
fn ends_with_unescaped_quote(s: &str, quote_char: char) -> bool {
    if !s.ends_with(quote_char) {
        return false;
    }

    let chars: Vec<char> = s.chars().collect();
    let mut i = chars.len();

    // Count consecutive backslashes before the final quote
    let mut backslash_count = 0;
    while i > 1 {
        i -= 1;
        if chars[i - 1] == '\\' {
            backslash_count += 1;
        } else {
            break;
        }
    }

    // If there's an even number of backslashes (including 0), the quote is not escaped
    backslash_count % 2 == 0
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

    #[test]
    #[serial]
    fn test_multiline_quoted_values() -> Result<()> {
        let input = r#"
        MESSAGE="This is line one
This is line two
This is line three"

        SINGLE_QUOTED='Multi
line
single quoted'

        REGULAR=single_line_value
    "#;

        let vars = parse_env_str(input)?;
        assert_eq!(
            vars.get("MESSAGE"),
            Some(&"This is line one\nThis is line two\nThis is line three".to_string())
        );
        assert_eq!(
            vars.get("SINGLE_QUOTED"),
            Some(&"Multi\nline\nsingle quoted".to_string())
        );
        assert_eq!(vars.get("REGULAR"), Some(&"single_line_value".to_string()));
        Ok(())
    }

    #[test]
    #[serial]
    fn test_escape_sequences() -> Result<()> {
        let input = r#"
        NEWLINES="Line 1\nLine 2\nLine 3"
        TABS="Col1\tCol2\tCol3"
        MIXED="Hello\tWorld\nSecond Line"
        ESCAPED_QUOTES="He said \"Hello World\""
        ESCAPED_BACKSLASH="Path\\to\\file"
    "#;

        let vars = parse_env_str(input)?;
        assert_eq!(
            vars.get("NEWLINES"),
            Some(&"Line 1\nLine 2\nLine 3".to_string())
        );
        assert_eq!(vars.get("TABS"), Some(&"Col1\tCol2\tCol3".to_string()));
        assert_eq!(
            vars.get("MIXED"),
            Some(&"Hello\tWorld\nSecond Line".to_string())
        );
        assert_eq!(
            vars.get("ESCAPED_QUOTES"),
            Some(&"He said \"Hello World\"".to_string())
        );
        assert_eq!(
            vars.get("ESCAPED_BACKSLASH"),
            Some(&"Path\\to\\file".to_string())
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn test_line_continuation() -> Result<()> {
        let input = r#"
        LONG_VALUE=This is a very \
long line that \
continues across \
multiple lines

        ANOTHER=single_line
    "#;

        let vars = parse_env_str(input)?;
        assert_eq!(
            vars.get("LONG_VALUE"),
            Some(&"This is a very long line that continues across multiple lines".to_string())
        );
        assert_eq!(vars.get("ANOTHER"), Some(&"single_line".to_string()));
        Ok(())
    }

    #[test]
    #[serial]
    fn test_comments_not_stripped_inside_quotes() -> Result<()> {
        let input = r#"
        COMMAND="echo 'Hello # World'"  # This is a real comment
        URL="https://example.com#section"
    "#;

        let vars = parse_env_str(input)?;
        assert_eq!(
            vars.get("COMMAND"),
            Some(&"echo 'Hello # World'".to_string())
        );
        assert_eq!(
            vars.get("URL"),
            Some(&"https://example.com#section".to_string())
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn test_sql_query_multiline() -> Result<()> {
        let input = r#"
        SQL_QUERY="SELECT users.name,
                          users.email,
                          posts.title
                   FROM users
                   LEFT JOIN posts ON users.id = posts.user_id
                   WHERE users.active = true"
    "#;

        let vars = parse_env_str(input)?;
        let expected = "SELECT users.name,\n                          users.email,\n                          posts.title\n                   FROM users\n                   LEFT JOIN posts ON users.id = posts.user_id\n                   WHERE users.active = true";
        assert_eq!(vars.get("SQL_QUERY"), Some(&expected.to_string()));
        Ok(())
    }

    #[test]
    #[serial]
    fn test_multiline_with_empty_lines() -> Result<()> {
        let input = r#"
        POEM="Roses are red

Violets are blue

This is a multiline
env variable"
    "#;

        let vars = parse_env_str(input)?;
        let expected = "Roses are red\n\nViolets are blue\n\nThis is a multiline\nenv variable";
        assert_eq!(vars.get("POEM"), Some(&expected.to_string()));
        Ok(())
    }

    #[test]
    #[serial]
    fn test_backward_compatibility() -> Result<()> {
        // Ensure all existing functionality still works
        let input = r#"
        # Comment
        SIMPLE=value
        QUOTED="quoted value"
        WITH_EQUALS=key=value
        SPACED = spaced value
        INLINE=value # comment
    "#;

        let vars = parse_env_str(input)?;
        assert_eq!(vars.get("SIMPLE"), Some(&"value".to_string()));
        assert_eq!(vars.get("QUOTED"), Some(&"quoted value".to_string()));
        assert_eq!(vars.get("WITH_EQUALS"), Some(&"key=value".to_string()));
        assert_eq!(vars.get("SPACED"), Some(&"spaced value".to_string()));
        assert_eq!(vars.get("INLINE"), Some(&"value".to_string()));
        Ok(())
    }
}
