use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Represents a single environment variable entry
#[derive(Debug, Clone, PartialEq)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
    pub line_number: usize,
}

/// Represents the difference between two .env files
#[derive(Debug)]
pub struct EnvDiff {
    pub added: Vec<EnvVar>,
    pub removed: Vec<EnvVar>,
    pub changed: Vec<ChangedVar>,
    pub unchanged: Vec<EnvVar>,
}

#[derive(Debug)]
pub struct ChangedVar {
    pub key: String,
    pub old_value: String,
    pub new_value: String,
}

/// Core struct for parsing, diffing, and merging .env files
pub struct EnvSync {
    vars: HashMap<String, EnvVar>,
}

impl EnvSync {
    /// Create an empty EnvSync
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    /// Parse a .env file from a file path
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        Self::from_str(&content)
    }

    /// Parse .env content from a string
    pub fn from_str(content: &str) -> Result<Self, String> {
        let mut sync = Self::new();
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Handle export prefix
            let line_content = if trimmed.starts_with("export ") {
                trimmed[7..].trim()
            } else {
                trimmed
            };

            // Parse KEY=VALUE
            if let Some(eq_pos) = line_content.find('=') {
                let key = line_content[..eq_pos].trim().to_string();
                let raw_value = line_content[eq_pos + 1..].trim();

                // Handle quoted values
                let value = Self::unquote(raw_value);

                sync.vars.insert(
                    key.clone(),
                    EnvVar {
                        key,
                        value,
                        line_number: line_num + 1,
                    },
                );
            }
        }
        Ok(sync)
    }

    /// Get all parsed environment variables
    pub fn vars(&self) -> &HashMap<String, EnvVar> {
        &self.vars
    }

    /// Get a specific variable value
    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(|v| v.value.as_str())
    }

    /// Compute the diff between this env and another
    pub fn diff(&self, other: &EnvSync) -> EnvDiff {
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut changed = Vec::new();
        let mut unchanged = Vec::new();

        // Find added and changed vars (in other but not in self, or different)
        for (key, other_var) in &other.vars {
            match self.vars.get(key) {
                Some(self_var) => {
                    if self_var.value == other_var.value {
                        unchanged.push(other_var.clone());
                    } else {
                        changed.push(ChangedVar {
                            key: key.clone(),
                            old_value: self_var.value.clone(),
                            new_value: other_var.value.clone(),
                        });
                    }
                }
                None => {
                    added.push(other_var.clone());
                }
            }
        }

        // Find removed vars (in self but not in other)
        for (key, self_var) in &self.vars {
            if !other.vars.contains_key(key) {
                removed.push(self_var.clone());
            }
        }

        EnvDiff {
            added,
            removed,
            changed,
            unchanged,
        }
    }

    /// Merge another env into this one. Values from `other` take precedence.
    pub fn merge(&mut self, other: &EnvSync) {
        for (key, var) in &other.vars {
            self.vars.insert(key.clone(), var.clone());
        }
    }

    /// Serialize the env vars to .env format string
    pub fn to_string_pretty(&self) -> String {
        let mut entries: Vec<&EnvVar> = self.vars.values().collect();
        entries.sort_by(|a, b| a.key.cmp(&b.key));

        entries
            .iter()
            .map(|var| format!("{}={}", var.key, Self::quote_value(&var.value)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Remove quotes from a value
    fn unquote(value: &str) -> String {
        let trimmed = value.trim();
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            trimmed[1..trimmed.len() - 1].to_string()
        } else {
            trimmed.to_string()
        }
    }

    /// Quote a value if it contains spaces or special characters
    fn quote_value(value: &str) -> String {
        if value.contains(' ') || value.contains('#') || value.contains('"') {
            format!("\"{}\"", value.replace('"', "\\\""))
        } else {
            value.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let env = EnvSync::from_str("FOO=bar\nBAZ=qux").unwrap();
        assert_eq!(env.get("FOO"), Some("bar"));
        assert_eq!(env.get("BAZ"), Some("qux"));
    }

    #[test]
    fn test_parse_comments_and_blanks() {
        let env = EnvSync::from_str("# comment\nFOO=bar\n\n# another\nBAZ=qux").unwrap();
        assert_eq!(env.vars().len(), 2);
    }

    #[test]
    fn test_parse_quoted_values() {
        let env = EnvSync::from_str("FOO=\"hello world\"\nBAR='single quoted'").unwrap();
        assert_eq!(env.get("FOO"), Some("hello world"));
        assert_eq!(env.get("BAR"), Some("single quoted"));
    }

    #[test]
    fn test_parse_export_prefix() {
        let env = EnvSync::from_str("export FOO=bar").unwrap();
        assert_eq!(env.get("FOO"), Some("bar"));
    }

    #[test]
    fn test_diff_added() {
        let base = EnvSync::from_str("FOO=bar").unwrap();
        let target = EnvSync::from_str("FOO=bar\nBAZ=qux").unwrap();
        let diff = base.diff(&target);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].key, "BAZ");
    }

    #[test]
    fn test_diff_removed() {
        let base = EnvSync::from_str("FOO=bar\nBAZ=qux").unwrap();
        let target = EnvSync::from_str("FOO=bar").unwrap();
        let diff = base.diff(&target);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0].key, "BAZ");
    }

    #[test]
    fn test_diff_changed() {
        let base = EnvSync::from_str("FOO=bar").unwrap();
        let target = EnvSync::from_str("FOO=baz").unwrap();
        let diff = base.diff(&target);
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].old_value, "bar");
        assert_eq!(diff.changed[0].new_value, "baz");
    }

    #[test]
    fn test_merge() {
        let mut base = EnvSync::from_str("FOO=bar").unwrap();
        let other = EnvSync::from_str("FOO=updated\nBAZ=new").unwrap();
        base.merge(&other);
        assert_eq!(base.get("FOO"), Some("updated"));
        assert_eq!(base.get("BAZ"), Some("new"));
    }

    #[test]
    fn test_to_string_pretty() {
        let env = EnvSync::from_str("Z=1\nA=2").unwrap();
        let output = env.to_string_pretty();
        assert!(output.starts_with("A=2"));
    }
}
