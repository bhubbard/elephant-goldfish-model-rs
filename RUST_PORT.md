# Elephant-Goldfish Model (EGM) - Native Rust Port

A native Rust implementation of Google's [Elephant-Goldfish Model (EGM)](https://github.com/google/elephant-goldfish-model) Spec-Driven Development (SDD) framework.

This port converts the packaging, release, and repository validation pipeline from Python to high-performance, memory-safe Rust, and expands EGM with a standalone command-line interface (`egm` / `goldfish`) and core library (`egm-core`) that can orchestrate Spec-Driven Development workflows directly from your terminal or CI environment.

---

## Workspace Architecture

The Rust workspace consists of two decoupled crates:

```text
goldfish/
├── Cargo.toml                  # Workspace manifest
├── crates/
│   ├── egm-core/               # Core domain logic, validation, packaging, & reviewer engine
│   │   ├── src/
│   │   │   ├── bootstrap.rs    # Ecosystem detection & documentation bootstrapper
│   │   │   ├── draft.rs        # Spec scaffolding (Feature, Heilmeier, Zoom-Out)
│   │   │   ├── drift.rs        # Spec-to-code living documentation drift analyzer
│   │   │   ├── manifest.rs     # Portable, Claude, & OpenAI plugin manifest models
│   │   │   ├── packaging.rs    # Deterministic ZIP packager & release manager
│   │   │   ├── personas.rs     # Goldfish reviewer personas & editorial rubrics
│   │   │   ├── review.rs       # Multi-stage Goldfish reviewer pipeline & synthesizer
│   │   │   ├── skills.rs       # Skill frontmatter parsing & Antigravity size limit checks
│   │   │   ├── standards.rs    # Document standards & evaluation rubrics
│   │   │   └── validator.rs    # Offline link integrity, table syntax, & manifest sync
│   │   └── tests/
│   └── egm-cli/                # Command-line binary (`egm` / `goldfish`)
│       └── src/
│           ├── lib.rs          # CLI argument parser & command dispatcher
│           ├── main.rs         # `egm` binary entrypoint
│           └── bin/goldfish.rs # `goldfish` binary alias
├── skills/                     # EGM agent skills
├── agents/                     # Goldfish reviewer personas
├── resources/                  # Document standards & templates
└── docs/                       # Architectural designs and guides
```

---

## Features

### 1. High-Performance Repository Validation (`egm check`)
Replaces Python offline tests with native checks running in sub-second time:
- **Manifest Synchronization**: Validates `plugin.json`, `.claude-plugin/plugin.json`, and `.codex-plugin/plugin.json` have synchronized versioning.
- **Skill Frontmatter & Limits**: Verifies all `skills/*/SKILL.md` files have valid YAML frontmatter and respect Antigravity's 24,000-byte body limit.
- **Link Integrity**: Checks relative links and intra-document anchors with GitHub-compatible slugification (double hyphens on stripped punctuation).
- **Markdown Tables**: Ensures no multiline continuation rows (`:`) that break on GitHub Flavored Markdown.
- **Editorial Rubrics**: Enforces `DROP`, `CALIBRATE`, `RELOCATE`, and `KEEP` actions in `agents/style.md`.

### 2. Deterministic Packaging & Release Preparation (`egm pack`, `egm release`)
Replaces `pack_plugin.py` and `prepare_release.py`:
- **Deterministic ZIPs**: Creates identical zip archives for Claude and Codex/OpenAI runtimes with Deflate compression and normalized permissions.
- **Security Invariants**: Enforces that runtime bundles contain no symlinks, no paths escaping the source checkout, and no disallowed MCP servers or hooks.
- **Release Automation**: Validates tag syntax (`vMAJOR.MINOR.PATCH`), hashes distribution archives with SHA-256 (`SHA256SUMS`), and prepares `RELEASE_NOTES.md`.

### 3. Living Documentation Drift Detection (`egm drift`)
Cross-references specification files (`docs/designs/*.md`) against actual codebase symbols, paths, and modules to flag orphaned promises or deleted implementations.

### 4. Goldfish Spec Review Engine (`egm review`)
Audits draft specifications against the 5-phase Goldfish review protocol:
- **Stage 1**: Problem statement & Heilmeier alignment
- **Stage 2**: Adversarial failure analysis & system scope
- **Stage 3**: Verification planning & rollback readiness
- **Stage 4**: Defensive caveat calibration
- **Stage 5**: Synthesizer report generation (Markdown or JSON)

### 5. Spec & Concept Scaffolding (`egm draft`, `egm brainstorm`)
- Rapidly generates structured specifications matching the **Feature Spec**, **Heilmeier Catechism**, or **Zoom-Out** standards.
- Scaffolds 6-lens shootdown concept briefs (Inversion, First Principles, Worst-Case, Simplicity, Prior Art, User-Centric).

---

## Installation & Quickstart

### Build the CLI

```bash
cargo build --release
```

The resulting binaries will be at `target/release/egm` (and alias `target/release/goldfish`).

### Run Offline Validation

```bash
cargo run -p egm-cli -- check
```

### Review a Design Document

```bash
cargo run -p egm-cli -- review docs/designs/architecture.md
```

### Scaffold a New Feature Spec

```bash
cargo run -p egm-cli -- draft "Payment System" --standard feature --output docs/designs/payment.md
```

### Package Distribution Bundles

```bash
cargo run -p egm-cli -- pack --output dist
```

### Run All Tests

```bash
cargo test --workspace
```

---

## License

Apache 2.0.
