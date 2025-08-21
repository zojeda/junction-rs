# Contributing to JunctionRS

Thanks for your interest in contributing! JunctionRS is early and exploratory. The focus is a lean, type-safe Object Graph Mapper (OGM) for Rust with a pluggable adapter architecture.

## 📌 Guiding Principles
- **Minimal core**: Keep abstractions small & ergonomic; avoid premature generalization.
- **Type safety first**: Prefer compile-time guarantees over runtime checks where feasible.
- **Async-first**: All public I/O paths should be async-friendly.
- **Backend agnostic**: Core should not leak adapter (SurrealDB) specifics.
- **Incremental evolution**: Start simple, iterate with real usage feedback.

## 🧱 Repository Layout (Quick Recap)
- `crates/junction-rs` – User-facing façade crate (re-exports core + macros + optional adapters).
- `crates/junction-rs-core` – Core traits, schema, ID types, DSL.
- `crates/junction-rs-macros` – Procedural derive macros (`Node`, `Edge`).
- `crates/junction-rs-surrealdb` – SurrealDB adapter (feature: `surreal`).

## 🚀 Getting Started (Dev Setup)
```bash
# Run from repo root
cargo check --all-features
cargo test --all --all-features
```

For quick manual validation:
```bash
cd crates/junction-rs
cargo run --example insert_person --features surreal
```

## 🔍 Testing
- Prefer adding focused tests in `crates/*/tests/`.
- Integration tests for adapter behavior live under the adapter crate (`junction-rs-surrealdb`).
- Keep tests deterministic (use in-memory DB / `mem://` endpoints for SurrealDB).

## 🧪 Adding a New Adapter (Early Guidance)
1. Create a new crate `junction-rs-<backend>`.
2. Implement the required `Adapter` trait pieces (see `junction-rs-core`).
3. Re-export behind a feature in `junction-rs` facade crate.
4. Provide smoke tests & minimal examples.

## 🧬 Style & Tooling
- Run `cargo fmt` before submitting.
- Fix (or justify) Clippy warnings: `cargo clippy --all-targets --all-features -- -D warnings`.
- Keep public API additions documented in README or inline rustdoc.

## 💬 Proposing Changes
1. Open an issue for larger feature ideas or design shifts.
2. For small fixes (typos, docs, minor improvements), feel free to open a PR directly.
3. Provide rationale in PR description—especially for API changes.

## 🧾 Commit & PR Conventions
- Use clear, imperative commit messages: `Add edge traversal helper`, `Refactor expression builder`, etc.
- Squash if the history is noisy.
- Reference issues with `Closes #NN` where applicable.

## 🔐 Licensing of Contributions
JunctionRS is dual-licensed under **MIT OR Apache-2.0**. By contributing, you agree that your contributions are provided under these terms. See `LICENSE`, `LICENSE-MIT`, and `LICENSE-APACHE`.

If you copy code from another source, ensure it is compatible with both MIT and Apache-2.0 and attribute appropriately (e.g., via NOTICE updates or comments).

## 🧾 NOTICE Updates
If you add significant new third-party code or dependencies that require attribution beyond normal Cargo metadata, update the `NOTICE` file.

## 🛑 Out of Scope (For Now)
- Complex migration tooling
- Full-text search abstractions
- Heavy code generation layers
- Multi-language bindings

## 🙌 Code of Conduct
A formal CoC is not yet included; please act with respect & professionalism. Harassment or hostile behavior is not tolerated.

## 📨 Need Help?
Open an issue with context, reproduction steps, and expected vs actual behavior.

Thanks again for helping shape JunctionRS early! ✨
