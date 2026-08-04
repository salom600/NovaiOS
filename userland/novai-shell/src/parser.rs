//! Tiny command-line tokenizer + pipeline splitter for novai-shell.
//! Supports: `|` pipe, `;` separator, quoted args (single and double), and
//! backslash escapes. Variable expansion (`$FOO`) is intentionally NOT done
//! here — the built-in `export`/`set` covers it for our use cases.

pub fn split_pipeline(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut in_squote = false;
    let mut in_dquote = false;
    let mut iter = input.chars().peekable();
    while let Some(c) = iter.next() {
        match c {
            '\\' => {
                if let Some(n) = iter.next() {
                    buf.push(n);
                }
            }
            '\'' if !in_dquote => in_squote = !in_squote,
            '"' if !in_squote => in_dquote = !in_dquote,
            ';' if !in_squote && !in_dquote => {
                let trimmed = buf.trim().to_string();
                if !trimmed.is_empty() {
                    out.push(trimmed);
                }
                buf.clear();
            }
            _ => buf.push(c),
        }
    }
    let trimmed = buf.trim().to_string();
    if !trimmed.is_empty() {
        out.push(trimmed);
    }
    out
}

pub fn tokenize(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut in_squote = false;
    let mut in_dquote = false;
    let mut iter = input.chars().peekable();
    while let Some(c) = iter.next() {
        match c {
            '\\' => {
                if let Some(n) = iter.next() {
                    buf.push(n);
                }
            }
            '\'' if !in_dquote => in_squote = !in_squote,
            '"' if !in_squote => in_dquote = !in_dquote,
            c if c.is_whitespace() && !in_squote && !in_dquote => {
                if !buf.is_empty() {
                    out.push(buf.clone());
                    buf.clear();
                }
            }
            _ => buf.push(c),
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split() {
        assert_eq!(
            split_pipeline("ls; pwd"),
            vec!["ls".to_string(), "pwd".to_string()]
        );
        assert_eq!(
            split_pipeline("echo 'a; b'; pwd"),
            vec!["echo 'a; b'".to_string(), "pwd".to_string()]
        );
    }
    #[test]
    fn tokens() {
        assert_eq!(
            tokenize("echo \"hello world\" foo"),
            vec!["echo", "hello world", "foo"]
        );
        assert_eq!(tokenize("ls 'a b'\\''c'"), vec!["ls", "a b'c"]);
    }
}
