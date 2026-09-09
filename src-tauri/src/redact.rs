use parking_lot::Mutex;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Redaction rule: regex → replacement. Applied to the text of every run
/// and to the displayed command line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactRule {
    pub pattern: String,
    pub replacement: String,
    pub enabled: bool,
    /// Default rules can be disabled but not deleted by the UI.
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Redaction {
    pub rules: Vec<RedactRule>,
}

/// Generates the default rules from the user's real environment.
pub fn default_rules() -> Vec<RedactRule> {
    let user = std::env::var("USER").unwrap_or_default();
    let home = std::env::var("HOME").unwrap_or_else(|_| format!("/home/{user}"));
    let hostname = hostname();
    vec![
        RedactRule {
            pattern: regex::escape(&home),
            replacement: "~".into(),
            enabled: true,
            is_default: true,
        },
        RedactRule {
            pattern: regex::escape(&user),
            replacement: "user".into(),
            enabled: true,
            is_default: true,
        },
        RedactRule {
            pattern: regex::escape(&hostname),
            replacement: "localhost".into(),
            enabled: true,
            is_default: true,
        },
    ]
}

fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

impl Redaction {
    /// Applies all enabled rules, in order, to a text.
    /// An invalid pattern is silently ignored (the UI validates separately).
    pub fn apply(&self, text: &str) -> String {
        let mut out = text.to_string();
        for rule in &self.rules {
            if !rule.enabled || rule.pattern.is_empty() {
                continue;
            }
            // Per-pattern cached compilation to avoid recompiling per run.
            if let Ok(re) = compile(&rule.pattern) {
                if re.is_match(&out) {
                    out = re.replace_all(&out, rule.replacement.as_str()).into_owned();
                }
            }
        }
        out
    }

    /// Validates all patterns; returns the indexes of the invalid ones.
    pub fn invalid_patterns(&self) -> Vec<usize> {
        self.rules
            .iter()
            .enumerate()
            .filter(|(_, r)| !r.pattern.is_empty() && Regex::new(&r.pattern).is_err())
            .map(|(i, _)| i)
            .collect()
    }
}

/// Cache of compiled regexes per pattern; avoids recompiling per run.
static RE_CACHE: LazyLock<Mutex<std::collections::HashMap<String, std::sync::Arc<Regex>>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

/// Compiles the pattern (cached). Returns Err if it doesn't compile.
fn compile(pattern: &str) -> Result<std::sync::Arc<Regex>, regex::Error> {
    let mut map = RE_CACHE.lock();
    if let Some(re) = map.get(pattern) {
        return Ok(re.clone());
    }
    let re = std::sync::Arc::new(Regex::new(pattern)?);
    map.insert(pattern.to_string(), re.clone());
    Ok(re)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(pairs: &[(&str, &str, bool)]) -> Redaction {
        Redaction {
            rules: pairs
                .iter()
                .map(|(p, r, e)| RedactRule {
                    pattern: p.to_string(),
                    replacement: r.to_string(),
                    enabled: *e,
                    is_default: false,
                })
                .collect(),
        }
    }

    #[test]
    fn home_becomes_tilde() {
        let r = rules(&[("/home/maria", "~", true)]);
        assert_eq!(r.apply("cd /home/maria/proyecto"), "cd ~/proyecto");
    }

    #[test]
    fn user_and_hostname() {
        let r = rules(&[("\\bmaria\\b", "user", true), ("mi-pc", "localhost", true)]);
        assert_eq!(
            r.apply("maria@mi-pc:~/repo (maria)"),
            "user@localhost:~/repo (user)"
        );
    }

    #[test]
    fn disabled_rules_skipped() {
        let r = rules(&[("secret", "X", false)]);
        assert_eq!(r.apply("secret token"), "secret token");
    }

    #[test]
    fn invalid_pattern_ignored() {
        let r = rules(&[("(unclosed", "X", true)]);
        assert_eq!(r.apply("(unclosed text"), "(unclosed text");
        assert_eq!(r.invalid_patterns().len(), 1);
    }

    #[test]
    fn applied_in_order() {
        let r = rules(&[("a", "b", true), ("b", "c", true)]);
        assert_eq!(r.apply("a"), "c");
    }

    #[test]
    fn command_line_is_just_text() {
        let r = rules(&[("/home/maria", "~", true)]);
        assert_eq!(r.apply("cat /home/maria/.zshrc"), "cat ~/.zshrc");
    }

    #[test]
    fn default_rules_valid() {
        let d = default_rules();
        assert_eq!(d.len(), 3);
        assert!(d.iter().all(|r| Regex::new(&r.pattern).is_ok()));
    }
}
