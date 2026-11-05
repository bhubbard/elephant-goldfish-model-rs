// Copyright 2026 Google LLC.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use colored::*;
use egm_core::{
    build_package, discover_skills, load_all_personas, prepare_release, Bootstrapper, CheckStatus,
    DocumentStandard, DraftGenerator, DriftAnalyzer, ReviewEngine, StandardType, Validator,
};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "egm",
    about = "Elephant-Goldfish Model (EGM) - Spec-Driven Development CLI",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Root directory of the repository or project
    #[arg(short, long, global = true, default_value = ".")]
    pub root: PathBuf,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run all repository offline validation checks
    Check {
        /// Verbose check reporting
        #[arg(short, long)]
        verbose: bool,
    },

    /// Build runtime folders and deterministic ZIP archives
    Pack {
        /// Packaging target: claude, openai, or all
        #[arg(short, long, default_value = "all")]
        target: PackTarget,

        /// Output destination directory
        #[arg(short, long, default_value = "dist")]
        output: PathBuf,
    },

    /// Prepare versioned release archives, checksums, and draft notes
    Release {
        /// Release tag in vMAJOR.MINOR.PATCH format
        #[arg(short, long)]
        tag: String,

        /// Full 40-character lowercase Git SHA commit
        #[arg(short, long)]
        commit: String,

        /// Output destination directory
        #[arg(short, long, default_value = "dist/release")]
        output: PathBuf,
    },

    /// Run the multi-stage Goldfish reviewer pipeline on a design spec
    Review {
        /// Path to design specification Markdown file
        spec: PathBuf,

        /// Output format (markdown or json)
        #[arg(short, long, default_value = "markdown")]
        format: OutputFormat,

        /// Optional file path to write the review report
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Scaffold a new specification document according to a standard
    Draft {
        /// Specification title
        title: String,

        /// Document standard template to use
        #[arg(short, long, default_value = "feature")]
        standard: StandardChoice,

        /// Author name
        #[arg(short, long, default_value = "Software Engineer")]
        author: String,

        /// Output file path
        #[arg(short, long, default_value = "design_draft.md")]
        output: PathBuf,

        /// Overwrite if file already exists
        #[arg(short, long)]
        force: bool,
    },

    /// Scaffold a 6-lens shootdown concept brief
    Brainstorm {
        /// Concept or feature idea description
        idea: String,

        /// Author name
        #[arg(short, long, default_value = "Software Engineer")]
        author: String,

        /// Output file path
        #[arg(short, long, default_value = "concepts_brief.md")]
        output: PathBuf,

        /// Overwrite if file already exists
        #[arg(short, long)]
        force: bool,
    },

    /// Analyze codebase and design documents for living specification drift
    Drift {
        /// Directory containing design specs
        #[arg(short, long, default_value = "docs/designs")]
        specs: PathBuf,
    },

    /// Inspect codebase and bootstrap living documentation architecture
    Bootstrap {
        /// Project name
        #[arg(short, long, default_value = "Project")]
        project: String,
    },

    /// List components: skills, personas, or standards
    List {
        /// Component category to list
        category: ListCategory,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackTarget {
    Claude,
    Openai,
    All,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Markdown,
    Json,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum StandardChoice {
    Feature,
    Heilmeier,
    ZoomOut,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListCategory {
    Skills,
    Personas,
    Standards,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let root = cli
        .root
        .canonicalize()
        .context("Failed to canonicalize root directory")?;

    match cli.command {
        Commands::Check { verbose } => run_check(&root, verbose),
        Commands::Pack { target, output } => run_pack(&root, target, &output),
        Commands::Release {
            tag,
            commit,
            output,
        } => run_release(&root, &tag, &commit, &output),
        Commands::Review {
            spec,
            format,
            output,
        } => run_review(&root, &spec, format, output.as_deref()),
        Commands::Draft {
            title,
            standard,
            author,
            output,
            force,
        } => run_draft(&root, &title, standard, &author, &output, force),
        Commands::Brainstorm {
            idea,
            author,
            output,
            force,
        } => run_brainstorm(&root, &idea, &author, &output, force),
        Commands::Drift { specs } => run_drift(&root, &specs),
        Commands::Bootstrap { project } => run_bootstrap(&root, &project),
        Commands::List { category } => run_list(&root, category),
    }
}

fn run_check(root: &Path, verbose: bool) -> Result<()> {
    println!(
        "{}",
        "Running Elephant-Goldfish Model (EGM) Validation Suite..."
            .bold()
            .cyan()
    );
    let validator = Validator::new(root);
    let report = validator.run_all_checks();

    let mut failed = false;

    for res in &report.results {
        match &res.status {
            CheckStatus::Passed => {
                println!("  {} {}", "✓".green().bold(), res.name);
            }
            CheckStatus::Warning(msg) => {
                println!("  {} {} - {}", "⚠".yellow().bold(), res.name, msg);
            }
            CheckStatus::Failed(err) => {
                failed = true;
                println!("  {} {}", "✗".red().bold(), res.name.bold());
                for line in err.lines() {
                    println!("    {}", line.red());
                }
            }
        }
    }

    if verbose {
        println!("\nChecked root: {}", root.display());
    }

    if failed {
        bail!("{}", "Validation suite failed!".red().bold());
    } else {
        println!("\n{}", "All checks passed successfully.".green().bold());
        Ok(())
    }
}

fn run_pack(root: &Path, target: PackTarget, output: &Path) -> Result<()> {
    let targets = match target {
        PackTarget::Claude => vec!["claude"],
        PackTarget::Openai => vec!["openai"],
        PackTarget::All => vec!["claude", "openai"],
    };

    println!("{}", "Packaging EGM runtime bundles...".bold().cyan());
    std::fs::create_dir_all(output)?;

    for t in targets {
        let (folder, archive) = build_package(root, t, output)
            .with_context(|| format!("Failed to build package for {}", t))?;
        println!(
            "  {} Target '{}': folder `{}` & archive `{}`",
            "✓".green().bold(),
            t,
            folder.display(),
            archive.display()
        );
    }

    println!("{}", "Packaging complete.".green().bold());
    Ok(())
}

fn run_release(root: &Path, tag: &str, commit: &str, output: &Path) -> Result<()> {
    println!(
        "{} Preparing release for tag {} (commit {})...",
        "→".cyan().bold(),
        tag.bold(),
        commit
    );

    let dest = prepare_release(root, tag, commit, output).context("Failed to prepare release")?;

    println!(
        "{} Release prepared successfully at `{}`",
        "✓".green().bold(),
        dest.display()
    );
    println!("  - Created versioned archives for Claude and OpenAI");
    println!("  - Generated SHA256SUMS");
    println!("  - Generated draft RELEASE_NOTES.md");

    Ok(())
}

fn run_review(root: &Path, spec: &Path, format: OutputFormat, output: Option<&Path>) -> Result<()> {
    let spec_path = if spec.is_absolute() {
        spec.to_path_buf()
    } else {
        root.join(spec)
    };

    if !spec_path.exists() {
        bail!("Specification file not found: {}", spec_path.display());
    }

    let report = ReviewEngine::review_spec_file(&spec_path).context("Review evaluation failed")?;

    let rendered = match format {
        OutputFormat::Markdown => report.to_markdown(),
        OutputFormat::Json => serde_json::to_string_pretty(&report)?,
    };

    if let Some(out) = output {
        let out_path = if out.is_absolute() {
            out.to_path_buf()
        } else {
            root.join(out)
        };
        std::fs::write(&out_path, &rendered)?;
        println!(
            "{} Review report saved to `{}`",
            "✓".green().bold(),
            out_path.display()
        );
    } else {
        println!("{}", rendered);
    }

    Ok(())
}

fn run_draft(
    root: &Path,
    title: &str,
    standard: StandardChoice,
    author: &str,
    output: &Path,
    force: bool,
) -> Result<()> {
    let std_type = match standard {
        StandardChoice::Feature => StandardType::FeatureSpec,
        StandardChoice::Heilmeier => StandardType::Heilmeier,
        StandardChoice::ZoomOut => StandardType::ZoomOut,
    };

    let content = DraftGenerator::scaffold_spec(title, std_type, author);
    let target_path = if output.is_absolute() {
        output.to_path_buf()
    } else {
        root.join(output)
    };

    DraftGenerator::write_to_file(&target_path, &content, force)
        .with_context(|| format!("Failed to scaffold draft to {}", target_path.display()))?;

    println!(
        "{} Created specification draft: `{}` ({})",
        "✓".green().bold(),
        target_path.display(),
        std_type.display_name()
    );

    Ok(())
}

fn run_brainstorm(root: &Path, idea: &str, author: &str, output: &Path, force: bool) -> Result<()> {
    let content = DraftGenerator::scaffold_brainstorm_brief(idea, author);
    let target_path = if output.is_absolute() {
        output.to_path_buf()
    } else {
        root.join(output)
    };

    DraftGenerator::write_to_file(&target_path, &content, force)
        .with_context(|| format!("Failed to scaffold brief to {}", target_path.display()))?;

    println!(
        "{} Created 6-lens concept brief: `{}`",
        "✓".green().bold(),
        target_path.display()
    );

    Ok(())
}

fn run_drift(root: &Path, specs: &Path) -> Result<()> {
    println!(
        "{}",
        "Scanning for living documentation drift...".bold().cyan()
    );
    let analyzer = DriftAnalyzer::new(root);
    let report = analyzer.scan_drift(specs)?;

    println!("{}", report.to_markdown());
    Ok(())
}

fn run_bootstrap(root: &Path, project: &str) -> Result<()> {
    println!(
        "{} Inspecting project at `{}`...",
        "→".cyan().bold(),
        root.display()
    );

    let inspection = Bootstrapper::inspect(root);
    println!("  Detected ecosystem: {:?}", inspection.ecosystem);
    println!("  Build command:      `{}`", inspection.build_command);
    println!("  Test command:       `{}`", inspection.test_command);

    let path = Bootstrapper::bootstrap_docs(root, project)?;
    println!(
        "{} Bootstrapped living specs in `{}`",
        "✓".green().bold(),
        path.display()
    );

    Ok(())
}

fn run_list(root: &Path, category: ListCategory) -> Result<()> {
    match category {
        ListCategory::Skills => {
            let skills = discover_skills(root.join("skills"))?;
            println!("{}", "Elephant-Goldfish Model (EGM) Skills:".bold().cyan());
            for s in skills {
                println!(
                    "  * {} ({}) - {} ({} bytes)",
                    s.frontmatter.name.bold(),
                    s.id.dimmed(),
                    s.frontmatter.description,
                    s.body_bytes
                );
            }
        }
        ListCategory::Personas => {
            let personas = load_all_personas(root.join("agents"))?;
            println!("{}", "Goldfish Reviewer Personas:".bold().cyan());
            for p in personas {
                println!(
                    "  * {} (`agents/{}.md`)",
                    p.role.display_name().bold(),
                    p.role.as_str()
                );
            }
        }
        ListCategory::Standards => {
            let standards =
                DocumentStandard::load_from_dir(root.join("resources/document_standards"))?;
            println!("{}", "Document Standards:".bold().cyan());
            for std in standards {
                println!(
                    "  * {} (`{}`)",
                    std.standard_type.display_name().bold(),
                    std.standard_type.as_filename()
                );
            }
        }
    }
    Ok(())
}
