//! Shared flags and helpers for release subcommands.

use std::fmt;
use std::str::FromStr;

use anyhow::anyhow;
use anyhow::Result;

/// A release line — either `tip` (plain `main`/`dev` pair) or an LTS line
/// identified by its `<major>.<minor>.x` prefix.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Line {
    Tip,
    Lts { major: u64, minor: u64 },
}

impl Line {
    /// Canonical identifier used in CLI flags, branch-name suffixes, and
    /// user-facing output.  `Tip` stringifies to `"tip"`; LTS stringifies to
    /// `"<major>.<minor>.x"`.
    pub fn id(&self) -> String {
        match self {
            Line::Tip => "tip".to_string(),
            Line::Lts { major, minor } => format!("{major}.{minor}.x"),
        }
    }

    /// The line's main (release-only) branch name on the remote.
    pub fn main_branch(&self) -> String {
        match self {
            Line::Tip => "main".to_string(),
            Line::Lts { major, minor } => format!("main-v{major}.{minor}.x"),
        }
    }

    /// The line's dev (development trunk) branch name on the remote.
    pub fn dev_branch(&self) -> String {
        match self {
            Line::Tip => "dev".to_string(),
            Line::Lts { major, minor } => format!("dev-v{major}.{minor}.x"),
        }
    }

    /// Whether this line is an LTS line (vs. tip).
    pub fn is_lts(&self) -> bool {
        matches!(self, Line::Lts { .. })
    }

    /// Try to parse a branch name of the form `main`, `dev`, `main-v<MAJ>.<MIN>.x`,
    /// or `dev-v<MAJ>.<MIN>.x`.  Returns `Some((Line, is_main))` if recognized,
    /// where `is_main` distinguishes the main-branch half from the dev-branch half.
    pub fn parse_branch(branch: &str) -> Option<(Line, bool)> {
        if branch == "main" {
            return Some((Line::Tip, true));
        }
        if branch == "dev" {
            return Some((Line::Tip, false));
        }

        let (prefix, is_main) = if let Some(rest) = branch.strip_prefix("main-v") {
            (rest, true)
        } else if let Some(rest) = branch.strip_prefix("dev-v") {
            (rest, false)
        } else {
            return None;
        };

        // Expect "<major>.<minor>.x"
        let without_x = prefix.strip_suffix(".x")?;
        let mut parts = without_x.splitn(2, '.');
        let major: u64 = parts.next()?.parse().ok()?;
        let minor: u64 = parts.next()?.parse().ok()?;

        Some((Line::Lts { major, minor }, is_main))
    }
}

impl FromStr for Line {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        if s == "tip" {
            return Ok(Line::Tip);
        }
        let without_x = s
            .strip_suffix(".x")
            .ok_or_else(|| anyhow!("line must be `tip` or `<major>.<minor>.x`: got {s:?}"))?;
        let mut parts = without_x.splitn(2, '.');
        let major: u64 = parts
            .next()
            .ok_or_else(|| anyhow!("missing major in line {s:?}"))?
            .parse()
            .map_err(|e| anyhow!("invalid major in line {s:?}: {e}"))?;
        let minor: u64 = parts
            .next()
            .ok_or_else(|| anyhow!("missing minor in line {s:?}"))?
            .parse()
            .map_err(|e| anyhow!("invalid minor in line {s:?}: {e}"))?;
        if parts.next().is_some() {
            return Err(anyhow!("line must be `<major>.<minor>.x`: got {s:?}"))
        }
        Ok(Line::Lts { major, minor })
    }
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.id())
    }
}

/// Flags common to every `release` subcommand.  Flatten into a subcommand's
/// clap struct with `#[command(flatten)]`.
#[derive(Debug, clap::Parser)]
pub struct ReleaseCommonArgs {
    /// Target GitHub repo in `owner/name` form.  Passed through to `gh` for
    /// all API calls.  Overridable to support private forks (e.g., for
    /// security releases).
    #[clap(long, default_value = "apollographql/router")]
    pub repo: String,

    /// Git remote name used for `git fetch`, `git push`, etc.
    #[clap(long, default_value = "origin")]
    pub origin: String,

    /// Which release line to operate on.  Use `tip` for the standard
    /// `main`/`dev` pair, or `<major>.<minor>.x` for an LTS line.
    #[clap(long)]
    pub line: Option<Line>,

    /// Disable interactive prompts.  If a required input is missing and
    /// `--non-interactive` is set, the command fails rather than prompting.
    #[clap(long)]
    pub non_interactive: bool,
}

impl ReleaseCommonArgs {
    /// Resolve the release line, either from the explicit `--line` flag or
    /// by inspecting the current git branch.  Returns `None` if neither
    /// source yields a recognizable line — callers decide whether to prompt
    /// (interactive) or error out (non-interactive).
    pub fn resolve_line(&self) -> Option<Line> {
        if let Some(line) = self.line.clone() {
            return Some(line);
        }
        let current = current_branch().ok()?;
        Line::parse_branch(&current).map(|(line, _)| line)
    }
}

/// Read the current git branch (`git rev-parse --abbrev-ref HEAD`).
pub fn current_branch() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "failed to read current git branch: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tip_round_trip() {
        let line = Line::Tip;
        assert_eq!(line.id(), "tip");
        assert_eq!(line.main_branch(), "main");
        assert_eq!(line.dev_branch(), "dev");
        assert!(!line.is_lts());
        assert_eq!(Line::from_str("tip").unwrap(), line);
    }

    #[test]
    fn lts_round_trip() {
        let line = Line::Lts { major: 2, minor: 10 };
        assert_eq!(line.id(), "2.10.x");
        assert_eq!(line.main_branch(), "main-v2.10.x");
        assert_eq!(line.dev_branch(), "dev-v2.10.x");
        assert!(line.is_lts());
        assert_eq!(Line::from_str("2.10.x").unwrap(), line);
    }

    #[test]
    fn parse_branch_tip() {
        assert_eq!(Line::parse_branch("main"), Some((Line::Tip, true)));
        assert_eq!(Line::parse_branch("dev"), Some((Line::Tip, false)));
    }

    #[test]
    fn parse_branch_lts() {
        assert_eq!(
            Line::parse_branch("main-v2.10.x"),
            Some((Line::Lts { major: 2, minor: 10 }, true))
        );
        assert_eq!(
            Line::parse_branch("dev-v2.10.x"),
            Some((Line::Lts { major: 2, minor: 10 }, false))
        );
    }

    #[test]
    fn parse_branch_rejects_garbage() {
        assert_eq!(Line::parse_branch(""), None);
        assert_eq!(Line::parse_branch("feature-foo"), None);
        assert_eq!(Line::parse_branch("main-v2.10"), None); // missing .x
        assert_eq!(Line::parse_branch("main-v2"), None);
        assert_eq!(Line::parse_branch("2.10.x"), None); // version branch, not a line branch
    }

    #[test]
    fn line_from_str_rejects_garbage() {
        assert!(Line::from_str("").is_err());
        assert!(Line::from_str("2.10").is_err()); // missing .x
        assert!(Line::from_str("2.10.5").is_err()); // too specific (version, not line)
        assert!(Line::from_str("v2.10.x").is_err()); // has v prefix
    }
}
