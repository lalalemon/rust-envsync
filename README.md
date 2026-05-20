# rust-envsync

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)](https://www.rust-lang.org/)
[![Crates.io](https://img.shields.io/crates/v/envsync)](https://crates.io/crates/envsync)

A CLI tool to sync `.env` files across environments with diffing, merging, and validation. Never manually compare `.env` files again.

## Features

- 🔍 **Diff** — See exactly what changed between two `.env` files
- 🔀 **Merge** — Combine `.env` files with precedence rules
- ✅ **Validate** — Check env vars against type rules (ports, URLs, emails, enums)
- 📋 **List** — Display all variables with optional line numbers and sorting
- 🎨 **Colored output** — Clear, human-readable terminal output
- 📦 **JSON output** — Machine-readable diff format for CI pipelines

## Installation

### From crates.io

```bash
cargo install envsync
```

### From source

```bash
git clone https://github.com/lalalemon/rust-envsync
cd rust-envsync
cargo install --path .
```

## Usage

### Diff two .env files

```bash
envsync diff --base .env --target .env.production
```

Output:
```
Diff: .env vs .env.production

  Added (2):
    + REDIS_URL=redis://prod:6379
    + LOG_LEVEL=warn

  Changed (1):
    ~ DATABASE_URL: postgres://localhost/dev → postgres://prod-server/prod

  = 5 variables unchanged
```

### JSON format (for CI)

```bash
envsync diff --base .env --target .env.production --format json
```

### Merge .env files

```bash
# Target values take precedence
envsync merge --base .env --target .env.production --output .env.merged
```

### Validate a .env file

```bash
# Use built-in common rules
envsync validate --file .env --common

# Check port numbers
envsync validate --file .env --check-ports

# Check URLs
envsync validate --file .env --check-urls
```

### List variables

```bash
envsync list --file .env --sort --line-numbers
```

## API Reference

### Parsing

```rust
use envsync::sync::EnvSync;
use std::path::Path;

// From file
let env = EnvSync::from_file(Path::new(".env"))?;

// From string
let env = EnvSync::from_str("FOO=bar\nBAZ=qux")?;

// Access variables
assert_eq!(env.get("FOO"), Some("bar"));
```

### Diffing

```rust
let base = EnvSync::from_file(Path::new(".env"))?;
let target = EnvSync::from_file(Path::new(".env.production"))?;

let diff = base.diff(&target);

for var in &diff.added {
    println!("Added: {}={}", var.key, var.value);
}
for var in &diff.removed {
    println!("Removed: {}={}", var.key, var.value);
}
for var in &diff.changed {
    println!("Changed: {} {} → {}", var.key, var.old_value, var.new_value);
}
```

### Merging

```rust
let mut base = EnvSync::from_file(Path::new(".env"))?;
let overrides = EnvSync::from_file(Path::new(".env.local"))?;

base.merge(&overrides); // overrides take precedence
println!("{}", base.to_string_pretty());
```

### Validation

```rust
use envsync::validate::{EnvValidator, ValidationRule};

let validator = EnvValidator::new()
    .rule("PORT", ValidationRule::Port)
    .rule("DATABASE_URL", ValidationRule::Required)
    .rule("NODE_ENV", ValidationRule::OneOf(vec![
        "development".into(),
        "production".into(),
    ]))
    .rule("DEBUG", ValidationRule::Boolean);

let env = EnvSync::from_file(Path::new(".env"))?;
let report = validator.validate(env.vars());

if !report.is_valid() {
    for err in report.errors() {
        println!("Error: {:?}", err.message);
    }
}
```

## Supported Validation Rules

| Rule | Description |
|------|-------------|
| `Required` | Value must not be empty |
| `Integer` | Must be a valid integer |
| `Float` | Must be a valid float |
| `Url` | Must start with `http://` or `https://` |
| `Email` | Must contain `@` and `.` |
| `OneOf(options)` | Must be one of the given values |
| `Prefix(s)` | Must start with the given prefix |
| `Boolean` | Must be `true`/`false`/`1`/`0`/`yes`/`no` |
| `Port` | Must be a valid port number (1-65535) |
| `CommaList` | Must be a non-empty comma-separated list |

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

<!-- history: 2026-05-16 -->

<!-- history: 2026-05-20 -->
