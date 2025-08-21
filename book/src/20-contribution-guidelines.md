# Contribution Guidelines & Style

## Communication
Open an issue for major proposals; small fixes or tests can go straight to PR.

## Code Style
- Run `cargo fmt --all`
- Run `cargo clippy --all --all-features -D warnings`

## Tests
Include tests for new features; mimic existing patterns in `crates/junction-rs/tests` and adapter tests.

## Documentation
Update the Book when adding notable public APIs. Keep examples minimal, focused.

## Commit Hygiene
- Descriptive messages
- Separate refactors from feature additions

## PR Scope
Prefer narrow changesets; large sweeping refactors risk churn in experimental project.

## Licensing
Dual-licensed under MIT/Apache-2.0; contributions follow same terms.

## Release Preparation
Ensure README and Book chapters reflect new features; update version and changelog (if present) before publishing.
