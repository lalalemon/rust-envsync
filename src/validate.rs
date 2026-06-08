use std::collections::HashMap;

use crate::sync::EnvVar;

/// Validation result for a single variable
#[derive(Debug)]
pub struct ValidationResult {
    pub key: String,
    pub valid: bool,
    pub message: Option<String>,
}

/// Validation report for an entire .env file
#[derive(Debug)]
pub struct ValidationReport {
    pub results: Vec<ValidationResult>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.results.iter().all(|r| r.valid)
    }

    pub fn errors(&self) -> Vec<&ValidationResult> {
        self.results.iter().filter(|r| !r.valid).collect()
    }
}

/// Validation rules for env vars
pub struct EnvValidator {
    rules: HashMap<String, ValidationRule>,
}

#[derive(Debug, Clone)]
pub enum ValidationRule {
    /// Value must not be empty
    Required,
    /// Value must be a valid integer
    Integer,
    /// Value must be a valid float
    Float,
    /// Value must be a valid URL
    Url,
    /// Value must be a valid email
    Email,
    /// Value must be one of the given options
    OneOf(Vec<String>),
    /// Value must match a prefix
    Prefix(String),
    /// Value must be a valid boolean (true/false/1/0)
    Boolean,
    /// Value must be a valid port number (1-65535)
    Port,
    /// Value must be a valid comma-separated list
    CommaList,
}

impl EnvValidator {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    /// Add a validation rule for a key
    pub fn rule(mut self, key: &str, rule: ValidationRule) -> Self {
        self.rules.insert(key.to_string(), rule);
        self
    }

    /// Add common validation rules for typical .env patterns
    pub fn with_common_rules(mut self) -> Self {
        let common: Vec<(&str, ValidationRule)> = vec![
            ("PORT", ValidationRule::Port),
            ("DATABASE_URL", ValidationRule::Required),
            ("NODE_ENV", ValidationRule::OneOf(vec![
                "development".into(),
                "production".into(),
                "test".into(),
            ])),
            ("DEBUG", ValidationRule::Boolean),
            ("SMTP_PORT", ValidationRule::Port),
        ];

        for (key, rule) in common {
            self.rules.insert(key.to_string(), rule);
        }

        self
    }

    /// Validate all variables against the configured rules
    pub fn validate(&self, vars: &HashMap<String, EnvVar>) -> ValidationReport {
        let mut results = Vec::new();

        for (key, rule) in &self.rules {
            match vars.get(key) {
                Some(var) => {
                    results.push(self.validate_var(key, &var.value, rule));
                }
                None => {
                    // Only report missing if rule is Required
                    if matches!(rule, ValidationRule::Required) {
                        results.push(ValidationResult {
                            key: key.clone(),
                            valid: false,
                            message: Some(format!("{} is required but not set", key)),
                        });
                    }
                }
            }
        }

        ValidationReport { results }
    }

    fn validate_var(&self, key: &str, value: &str, rule: &ValidationRule) -> ValidationResult {
        let (valid, message) = match rule {
            ValidationRule::Required => {
                (!value.is_empty(), Some(format!("{} must not be empty", key)))
            }
            ValidationRule::Integer => (
                value.parse::<i64>().is_ok(),
                Some(format!("{} must be a valid integer, got '{}'", key, value)),
            ),
            ValidationRule::Float => (
                value.parse::<f64>().is_ok(),
                Some(format!("{} must be a valid number, got '{}'", key, value)),
            ),
            ValidationRule::Url => (
                value.starts_with("http://") || value.starts_with("https://"),
                Some(format!("{} must be a valid URL, got '{}'", key, value)),
            ),
            ValidationRule::Email => (
                value.contains('@') && value.contains('.'),
                Some(format!("{} must be a valid email, got '{}'", key, value)),
            ),
            ValidationRule::OneOf(options) => (
                options.iter().any(|o| o == value),
                Some(format!(
                    "{} must be one of [{}], got '{}'",
                    key,
                    options.join(", "),
                    value
                )),
            ),
            ValidationRule::Prefix(prefix) => (
                value.starts_with(prefix),
                Some(format!("{} must start with '{}', got '{}'", key, prefix, value)),
            ),
            ValidationRule::Boolean => {
                let lower = value.to_lowercase();
                (
                    ["true", "false", "1", "0", "yes", "no"].contains(&lower.as_str()),
                    Some(format!("{} must be a boolean, got '{}'", key, value)),
                )
            }
            ValidationRule::Port => match value.parse::<u16>() {
                Ok(p) => (p > 0, Some(format!("{} must be a valid port number", key))),
                Err(_) => (
                    false,
                    Some(format!("{} must be a valid port number, got '{}'", key, value)),
                ),
            },
            ValidationRule::CommaList => (
                !value.is_empty() && value.split(',').all(|v| !v.trim().is_empty()),
                Some(format!("{} must be a comma-separated list", key)),
            ),
        };

        ValidationResult {
            key: key.to_string(),
            valid,
            message: if valid { None } else { message },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::EnvSync;

    #[test]
    fn test_validate_integer() {
        let validator = EnvValidator::new().rule("PORT", ValidationRule::Integer);
        let env = EnvSync::from_str("PORT=3000").unwrap();
        let report = validator.validate(env.vars());
        assert!(report.is_valid());
    }

    #[test]
    fn test_validate_integer_fail() {
        let validator = EnvValidator::new().rule("PORT", ValidationRule::Integer);
        let env = EnvSync::from_str("PORT=abc").unwrap();
        let report = validator.validate(env.vars());
        assert!(!report.is_valid());
    }

    #[test]
    fn test_validate_port() {
        let validator = EnvValidator::new().rule("PORT", ValidationRule::Port);
        let env = EnvSync::from_str("PORT=8080").unwrap();
        assert!(validator.validate(env.vars()).is_valid());
    }

    #[test]
    fn test_validate_port_zero() {
        let validator = EnvValidator::new().rule("PORT", ValidationRule::Port);
        let env = EnvSync::from_str("PORT=0").unwrap();
        assert!(!validator.validate(env.vars()).is_valid());
    }

    #[test]
    fn test_validate_email() {
        let validator = EnvValidator::new().rule("EMAIL", ValidationRule::Email);
        let env = EnvSync::from_str("EMAIL=user@example.com").unwrap();
        assert!(validator.validate(env.vars()).is_valid());
    }

    #[test]
    fn test_validate_url() {
        let validator = EnvValidator::new().rule("URL", ValidationRule::Url);
        let env = EnvSync::from_str("URL=https://example.com").unwrap();
        assert!(validator.validate(env.vars()).is_valid());
    }

    #[test]
    fn test_validate_one_of() {
        let validator = EnvValidator::new().rule(
            "NODE_ENV",
            ValidationRule::OneOf(vec!["development".into(), "production".into()]),
        );
        let env = EnvSync::from_str("NODE_ENV=development").unwrap();
        assert!(validator.validate(env.vars()).is_valid());
    }

    #[test]
    fn test_validate_boolean() {
        let validator = EnvValidator::new().rule("DEBUG", ValidationRule::Boolean);
        assert!(validator.validate(EnvSync::from_str("DEBUG=true").unwrap().vars()).is_valid());
        assert!(validator.validate(EnvSync::from_str("DEBUG=0").unwrap().vars()).is_valid());
        assert!(!validator.validate(EnvSync::from_str("DEBUG=maybe").unwrap().vars()).is_valid());
    }

    #[test]
    fn test_common_rules() {
        let validator = EnvValidator::new().with_common_rules();
        let env = EnvSync::from_str("PORT=3000\nDATABASE_URL=postgres://localhost/db\nNODE_ENV=development").unwrap();
        let report = validator.validate(env.vars());
        assert!(report.is_valid());
    }
}
