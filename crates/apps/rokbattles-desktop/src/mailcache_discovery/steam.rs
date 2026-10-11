use std::path::PathBuf;

/// Read just the library paths from Steam's KeyValues file. The input size is
/// bounded by the caller; nesting is iterative and capped, including unknown keys.
pub(super) fn library_paths(input: &str) -> Result<Vec<PathBuf>, ()> {
    let mut tokens = Tokens(input.trim_start_matches('\u{feff}').chars().peekable());
    let mut parents = Vec::new();
    let mut paths = Vec::new();

    while let Some(token) = tokens.next()? {
        let key = match token {
            Token::Text(key) => key,
            Token::Close => {
                parents.pop().ok_or(())?;
                continue;
            }
            Token::Open => return Err(()),
        };

        match tokens.next()?.ok_or(())? {
            Token::Open => {
                if parents.len() >= 16 {
                    return Err(());
                }
                parents.push(key);
            }
            Token::Text(value) => {
                let is_library_path = match parents.as_slice() {
                    [root, index] => {
                        root.eq_ignore_ascii_case("libraryfolders")
                            && is_index(index)
                            && key == "path"
                    }
                    [root] => root.eq_ignore_ascii_case("libraryfolders") && is_index(&key),
                    _ => false,
                };

                if is_library_path {
                    let path = PathBuf::from(value);
                    if path.is_absolute() {
                        paths.push(path);
                    }
                }
            }
            Token::Close => return Err(()),
        }
    }

    if !parents.is_empty() {
        return Err(());
    }

    Ok(paths)
}

fn is_index(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

enum Token {
    Text(String),
    Open,
    Close,
}

struct Tokens<'a>(std::iter::Peekable<std::str::Chars<'a>>);

impl Tokens<'_> {
    fn next(&mut self) -> Result<Option<Token>, ()> {
        loop {
            let Some(ch) = self.0.next() else {
                return Ok(None);
            };

            match ch {
                ch if ch.is_whitespace() => continue,
                '/' if self.0.peek() == Some(&'/') => {
                    for ch in self.0.by_ref() {
                        if ch == '\n' {
                            break;
                        }
                    }
                }
                '{' => return Ok(Some(Token::Open)),
                '}' => return Ok(Some(Token::Close)),
                '"' => {
                    let mut value = String::new();
                    loop {
                        match self.0.next().ok_or(())? {
                            '"' => return Ok(Some(Token::Text(value))),
                            '\\' => {
                                let escaped = self.0.next().ok_or(())?;
                                if !matches!(escaped, '"' | '\\') {
                                    value.push('\\');
                                }
                                value.push(escaped);
                            }
                            ch => value.push(ch),
                        }
                    }
                }
                ch => {
                    let mut value = String::from(ch);
                    while self
                        .0
                        .peek()
                        .is_some_and(|ch| !ch.is_whitespace() && !matches!(ch, '{' | '}' | '"'))
                    {
                        if let Some(ch) = self.0.next() {
                            value.push(ch);
                        }
                    }
                    return Ok(Some(Token::Text(value)));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_modern_and_legacy_libraries_without_confusing_nested_app_values() {
        let input = r#"// Steam libraries
        "libraryfolders" {
          "0" { "path" "/games/Steam Library" "apps" { "12" "999" } }
          "1" "/mnt/older library"
          "2" { "path" "/games/escaped\\name" }
          "3" { "path" "relative" }
        }"#;

        assert_eq!(
            library_paths(input).expect("valid libraries"),
            vec![
                PathBuf::from("/games/Steam Library"),
                PathBuf::from("/mnt/older library"),
                PathBuf::from("/games/escaped\\name"),
            ]
        );
    }

    #[test]
    fn rejects_unbalanced_and_deep_metadata() {
        for input in ["libraryfolders { 0", "}", "key \"unterminated", &"key { ".repeat(1000)] {
            library_paths(input).expect_err("malformed libraries");
        }
    }
}
