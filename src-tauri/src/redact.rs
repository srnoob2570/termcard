use parking_lot::Mutex;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Regla de redacción: regex → reemplazo. Se aplica al texto de cada run
/// y a la línea de comando mostrada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactRule {
    pub pattern: String,
    pub replacement: String,
    pub enabled: bool,
    /// Las reglas por defecto se pueden desactivar pero no borrar por la UI.
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Redaction {
    pub rules: Vec<RedactRule>,
}

/// Genera las reglas por defecto a partir del entorno real del usuario.
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
            replacement: "usuario".into(),
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
    /// Aplica todas las reglas habilitadas, en orden, sobre un texto.
    /// Un patrón inválido se ignora silenciosamente (la UI valida aparte).
    pub fn apply(&self, text: &str) -> String {
        let mut out = text.to_string();
        for rule in &self.rules {
            if !rule.enabled || rule.pattern.is_empty() {
                continue;
            }
            // Compilación cacheada por patrón para no recompilar por run.
            if let Ok(re) = compile(&rule.pattern) {
                if re.is_match(&out) {
                    out = re.replace_all(&out, rule.replacement.as_str()).into_owned();
                }
            }
        }
        out
    }

    /// Valida todos los patrones; devuelve los índices de los inválidos.
    pub fn invalid_patterns(&self) -> Vec<usize> {
        self.rules
            .iter()
            .enumerate()
            .filter(|(_, r)| !r.pattern.is_empty() && Regex::new(&r.pattern).is_err())
            .map(|(i, _)| i)
            .collect()
    }
}

/// Cache de regex compiladas por patrón; evita recompilar por run.
static RE_CACHE: LazyLock<Mutex<std::collections::HashMap<String, std::sync::Arc<Regex>>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

/// Compila el patrón (cacheado). Devuelve Err si no compila.
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
        let r = rules(&[
            ("\\bmaria\\b", "usuario", true),
            ("mi-pc", "localhost", true),
        ]);
        assert_eq!(
            r.apply("maria@mi-pc:~/repo (maria)"),
            "usuario@localhost:~/repo (usuario)"
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
