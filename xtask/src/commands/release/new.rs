//! `cargo xtask release new` — cut a fresh release branch and open the
//! draft release PR.
//!
//! Per-line behavior:
//!
//! - **Tip patch** (`--line tip --patch`): cut from the line's latest
//!   release tag (NOT from `dev`).  `dev` typically has post-release
//!   work that shouldn't ship in a patch.
//! - **Tip minor / major**: cut from `dev`.
//! - **LTS patch** (`--line 2.10.x --patch`): cut from the line's dev
//!   branch (e.g., `dev-v2.10.x`), since LTS dev IS the locked
//!   maintenance trunk — everything on it is already a backport.
//! - **Staging line** (`--line 3.x --patch` or first release): cut from
//!   `dev-v3.x`.
//!
//! Always adds a `--allow-empty` commit if the cut branch has no diff
//! against the line's main branch (otherwise GitHub refuses to open
//! the PR).

use std::str::FromStr;

use anyhow::Result;
use anyhow::anyhow;
use dialoguer::Confirm;
use dialoguer::theme::ColorfulTheme;
use semver::Version;
use xtask::*;

use super::common::Line;
use super::common::ReleaseCommonArgs;
use super::state;

/// Either a kind of bump (`patch`/`minor`/`major`) or a specific version.
#[derive(Debug, Clone)]
pub enum NewVersion {
    Patch,
    Minor,
    Major,
    Specific(Version),
}

impl FromStr for NewVersion {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "patch" => Ok(NewVersion::Patch),
            "minor" => Ok(NewVersion::Minor),
            "major" => Ok(NewVersion::Major),
            v => {
                let parsed = Version::parse(v).map_err(|e| {
                    anyhow!("invalid version or kind {v:?}: {e}; expected `patch`, `minor`, `major`, or a semver like `2.14.1`")
                })?;
                Ok(NewVersion::Specific(parsed))
            }
        }
    }
}

/// Cut a new release branch and open the draft release PR.
#[derive(Debug, clap::Parser)]
pub struct New {
    #[command(flatten)]
    pub common: ReleaseCommonArgs,

    /// `patch`, `minor`, `major`, or a specific version like `2.14.1`.
    pub version: NewVersion,

    /// Override the starting commit-ish.  Defaults differ by line:
    ///   - Tip patch       → latest release tag (e.g., `v2.14.0`)
    ///   - Tip minor/major → `dev`
    ///   - LTS patch       → line's dev branch (e.g., `dev-v2.10.x`)
    ///   - Staging         → line's dev branch
    #[clap(long)]
    pub from: Option<String>,

    /// Skip the empty-commit shim even when there's no diff.  The PR open
    /// will fail in that case — only useful when you know the cut already
    /// has commits the line's main doesn't.
    #[clap(long)]
    pub no_empty_commit_shim: bool,

    /// Print the exact commands without executing.
    #[clap(long)]
    pub dry_run: bool,
}

impl New {
    pub fn run(&self) -> Result<()> {
        let line = self.resolve_line()?;
        let target = self.resolve_target_version(&line)?;
        let from = self.resolve_from(&line, &self.version)?;

        eprintln!(
            "Cut new release: v{target} on line `{line}`  (base: `{from}`, target main: `{main}`)",
            main = line.main_branch()
        );

        if self.dry_run {
            eprintln!("(dry-run) — no commands will execute");
        }

        if !self.dry_run {
            self.ensure_pristine_checkout()?;
            self.check_no_existing_release_pr(&target)?;
        }

        if !self.common.non_interactive && !self.dry_run {
            let proceed = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(format!(
                    "Cut `{target}` from `{from}` and open draft release PR into `{main}`?",
                    main = line.main_branch()
                ))
                .default(true)
                .interact()?;
            if !proceed {
                eprintln!("Aborted by user.");
                return Ok(());
            }
        }

        self.cut_branch(&line, &target, &from)?;
        self.maybe_add_empty_commit(&line, &target)?;
        self.push_branch(&target)?;
        self.open_release_pr(&line, &target)?;

        Ok(())
    }

    /// Resolve the line — flag, current branch, or wizard.
    fn resolve_line(&self) -> Result<Line> {
        if let Some(line) = self.common.resolve_line() {
            return Ok(line);
        }
        if self.common.non_interactive {
            return Err(anyhow!(
                "could not infer release line; pass --line tip or --line <major>.<minor>.x"
            ));
        }
        // Default to tip when interactive but no line specified.  Most common case.
        Ok(Line::Tip)
    }

    /// Resolve the target version: explicit `--version` semver, or compute
    /// from `--patch`/`--minor`/`--major` against the line's latest tag.
    fn resolve_target_version(&self, line: &Line) -> Result<Version> {
        match &self.version {
            NewVersion::Specific(v) => Ok(v.clone()),
            kind => {
                let latest = self.latest_release_on_line(line)?.ok_or_else(|| {
                    anyhow!(
                        "no prior release tag found on line `{line}` to bump from; \
                         pass an explicit version like `2.14.1` instead of `{:?}`",
                        kind
                    )
                })?;
                bump(&latest, kind)
            }
        }
    }

    /// Read the latest released (non-pre-release) tag on the line from local refs.
    /// Assumes `git fetch --tags` has run recently.
    fn latest_release_on_line(&self, line: &Line) -> Result<Option<Version>> {
        let output = std::process::Command::new(which::which("git")?)
            .args(["tag", "--list", "v*"])
            .output()?;
        if !output.status.success() {
            return Err(anyhow!("git tag --list failed"));
        }
        let latest = String::from_utf8(output.stdout)?
            .lines()
            .filter_map(|s| s.strip_prefix('v'))
            .filter_map(|s| Version::parse(s).ok())
            .filter(|v| v.pre.is_empty())
            .filter(|v| line_claims_version(line, v))
            .max();
        Ok(latest)
    }

    /// Resolve `--from`: explicit override, or line-specific default.
    fn resolve_from(&self, line: &Line, kind: &NewVersion) -> Result<String> {
        if let Some(from) = &self.from {
            return Ok(from.clone());
        }
        // Line-specific defaults.
        let default = match (line, kind) {
            // Tip patch: from the latest released tag.  Dev moves too fast
            // for patches — use the released commit as the base.
            (Line::Tip, NewVersion::Patch) => {
                let latest = self.latest_release_on_line(line)?.ok_or_else(|| {
                    anyhow!(
                        "no prior release tag on tip; can't default --from for a patch.  \
                         Pass --from <commitish> explicitly."
                    )
                })?;
                format!("v{latest}")
            }
            // Tip minor or major: from dev.
            (Line::Tip, _) => line.dev_branch(),
            // LTS or staging: always from the line's dev branch.
            // For LTS that's already the locked-down trunk; for staging it's
            // the only thing that exists yet.
            (_, _) => line.dev_branch(),
        };
        Ok(default)
    }

    fn ensure_pristine_checkout(&self) -> Result<()> {
        let output = std::process::Command::new(which::which("git")?)
            .args(["status", "--untracked-files=no", "--porcelain"])
            .output()?;
        if !output.stdout.is_empty() {
            return Err(anyhow!(
                "git workspace is not clean; commit or stash changes before cutting a new release"
            ));
        }
        Ok(())
    }

    fn check_no_existing_release_pr(&self, version: &Version) -> Result<()> {
        if !xtask::gh::available() {
            return Err(anyhow!(
                "the `gh` CLI is required for release PR detection (install from https://cli.github.com/)"
            ));
        }
        let head = version.to_string();
        let prs: Vec<state::PrSummary> = xtask::gh::json([
            "--repo",
            self.common.repo.as_str(),
            "pr",
            "list",
            "--state",
            "open",
            "--head",
            head.as_str(),
            "--json",
            "number,title,headRefName,isDraft,mergeStateStatus,url",
        ])?;
        if let Some(pr) = prs.first() {
            return Err(anyhow!(
                "release PR for v{version} already exists: #{} {}; close or finish it before cutting a new one",
                pr.number,
                pr.url.as_deref().unwrap_or(""),
            ));
        }
        Ok(())
    }

    fn cut_branch(&self, line: &Line, version: &Version, from: &str) -> Result<()> {
        let branch = version.to_string();
        if self.dry_run {
            eprintln!("(dry-run) would run:");
            eprintln!(
                "  git fetch --tags --prune {origin}",
                origin = self.common.origin
            );
            eprintln!("  git checkout {from}");
            eprintln!("  git checkout -b {branch}");
            return Ok(());
        }
        let _ = (line,); // suppress unused warning until we add line-specific behavior
        git!(["fetch", "--tags", "--prune", self.common.origin.as_str()]);
        git!(["checkout", from]);
        git!(["checkout", "-b", branch.as_str()]);
        Ok(())
    }

    fn maybe_add_empty_commit(&self, line: &Line, version: &Version) -> Result<()> {
        if self.no_empty_commit_shim {
            eprintln!("(--no-empty-commit-shim) skipping shim check");
            return Ok(());
        }
        let main = line.main_branch();
        let branch = version.to_string();
        let range = format!(
            "{origin}/{main}..HEAD",
            origin = self.common.origin,
            main = main
        );
        if self.dry_run {
            eprintln!("(dry-run) would check:");
            eprintln!("  git rev-list {range} --count");
            eprintln!("  if 0:");
            eprintln!("    git commit --allow-empty -m \"Start v{version} PR\"");
            return Ok(());
        }
        let _ = (branch,);
        let count_output = std::process::Command::new(which::which("git")?)
            .args(["rev-list", "--count", range.as_str()])
            .output()?;
        if !count_output.status.success() {
            return Err(anyhow!(
                "failed to count commits between {origin}/{main} and HEAD: {}",
                String::from_utf8_lossy(&count_output.stderr).trim(),
                origin = self.common.origin,
            ));
        }
        let count: usize = String::from_utf8(count_output.stdout)?.trim().parse()?;
        if count == 0 {
            eprintln!(
                "no commits between {origin}/{main} and HEAD — adding empty commit so the PR can be opened",
                origin = self.common.origin,
            );
            git!([
                "commit",
                "--allow-empty",
                "-m",
                format!("Start v{version} PR").as_str(),
            ]);
        } else {
            eprintln!(
                "branch already has {count} commit(s) ahead of {origin}/{main} — no empty commit needed",
                origin = self.common.origin,
            );
        }
        Ok(())
    }

    fn push_branch(&self, version: &Version) -> Result<()> {
        let branch = version.to_string();
        if self.dry_run {
            eprintln!(
                "(dry-run) would run:\n  git push --set-upstream {origin} {branch}",
                origin = self.common.origin
            );
            return Ok(());
        }
        git!([
            "push",
            "--set-upstream",
            self.common.origin.as_str(),
            branch.as_str(),
        ]);
        Ok(())
    }

    fn open_release_pr(&self, line: &Line, version: &Version) -> Result<()> {
        let main = line.main_branch();
        let branch = version.to_string();
        let title = format!("release: v{version}");
        let body = release_pr_body(line, version);

        if self.dry_run {
            eprintln!("(dry-run) would run:");
            eprintln!(
                "  gh pr create --repo {repo} --draft --label release \\\n    \
                 -B {main} -H {branch} \\\n    \
                 --title \"{title}\" \\\n    \
                 --body <body>",
                repo = self.common.repo,
            );
            return Ok(());
        }

        if !xtask::gh::available() {
            return Err(anyhow!("the `gh` CLI is required to open the release PR"));
        }
        xtask::gh::run([
            "--repo",
            self.common.repo.as_str(),
            "pr",
            "create",
            "--draft",
            "--label",
            "release",
            "-B",
            main.as_str(),
            "-H",
            branch.as_str(),
            "--title",
            title.as_str(),
            "--body",
            body.as_str(),
        ])?;
        Ok(())
    }
}

/// Bump a version by kind.  Major bumps reset minor + patch; minor bumps
/// reset patch.  Patch increments patch.
fn bump(latest: &Version, kind: &NewVersion) -> Result<Version> {
    let mut v = latest.clone();
    match kind {
        NewVersion::Major => {
            v.major += 1;
            v.minor = 0;
            v.patch = 0;
        }
        NewVersion::Minor => {
            v.minor += 1;
            v.patch = 0;
        }
        NewVersion::Patch => {
            v.patch += 1;
        }
        NewVersion::Specific(_) => {
            return Err(anyhow!("internal: bump() called with NewVersion::Specific"));
        }
    }
    v.pre = semver::Prerelease::EMPTY;
    v.build = semver::BuildMetadata::EMPTY;
    Ok(v)
}

/// Whether `version` belongs to `line`.  Tip claims everything that no
/// LTS or staging line claims; LTS claims exact major+minor; staging
/// claims any major match (typically only pre-releases pre-first-minor).
fn line_claims_version(line: &Line, v: &Version) -> bool {
    match line {
        Line::Tip => true, // simplification: tip claims any tag for the bump-base lookup
        Line::Lts { major, minor } => v.major == *major && v.minor == *minor,
        Line::Staging { major } => v.major == *major,
    }
}

fn release_pr_body(line: &Line, version: &Version) -> String {
    let main = line.main_branch();
    format!(
"> **Note**
> **This particular PR must be true-merged to `{main}`.**

* This PR is only ready to review when it is marked as \"Ready for Review\".  It represents the merge to the `{main}` branch of an upcoming v{version} release.
* It will act as a staging branch until we are ready to finalize the release.
* We may cut any number of alpha and release candidate (RC) versions off this branch prior to formalizing it.
* Backports land here via Mergify (use the `backport-{version}` label on the source PR).
* This PR is **primarily a merge commit**, so reviewing every individual commit shown below is **not necessary** since those have been reviewed in their own PR.  However, things important to review on this PR **once it's marked \"Ready for Review\"**:
    - Does this PR target the right branch? (should be `{main}`)
    - Are the appropriate **version bumps** and **release note edits** in the end of the commit list (or within the last few commits).  In other words, \"Did the 'release prep' PR actually land on this branch?\"
    - If those things look good, this PR is good to merge!
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn bump_patch() {
        assert_eq!(bump(&v("2.14.0"), &NewVersion::Patch).unwrap(), v("2.14.1"));
        assert_eq!(bump(&v("2.10.2"), &NewVersion::Patch).unwrap(), v("2.10.3"));
    }

    #[test]
    fn bump_minor() {
        assert_eq!(bump(&v("2.14.0"), &NewVersion::Minor).unwrap(), v("2.15.0"));
        // Minor resets patch to 0.
        assert_eq!(bump(&v("2.14.5"), &NewVersion::Minor).unwrap(), v("2.15.0"));
    }

    #[test]
    fn bump_major() {
        assert_eq!(bump(&v("2.14.0"), &NewVersion::Major).unwrap(), v("3.0.0"));
        // Major resets minor + patch to 0.
        assert_eq!(bump(&v("2.14.5"), &NewVersion::Major).unwrap(), v("3.0.0"));
    }

    #[test]
    fn bump_strips_prerelease_metadata() {
        // If the latest tag was a pre-release somehow, bumping should land on
        // a clean release version.
        let pre = v("2.14.0-rc.2");
        assert_eq!(bump(&pre, &NewVersion::Patch).unwrap(), v("2.14.1"));
    }

    #[test]
    fn newversion_parses_kinds() {
        assert!(matches!(
            NewVersion::from_str("patch").unwrap(),
            NewVersion::Patch
        ));
        assert!(matches!(
            NewVersion::from_str("minor").unwrap(),
            NewVersion::Minor
        ));
        assert!(matches!(
            NewVersion::from_str("major").unwrap(),
            NewVersion::Major
        ));
    }

    #[test]
    fn newversion_parses_specific() {
        match NewVersion::from_str("2.14.1").unwrap() {
            NewVersion::Specific(ver) => assert_eq!(ver, v("2.14.1")),
            _ => panic!("expected Specific"),
        }
    }

    #[test]
    fn newversion_rejects_garbage() {
        assert!(NewVersion::from_str("").is_err());
        assert!(NewVersion::from_str("not-a-version").is_err());
        assert!(NewVersion::from_str("2.14").is_err());
    }

    #[test]
    fn line_claims_lts() {
        let lts = Line::Lts {
            major: 2,
            minor: 10,
        };
        assert!(line_claims_version(&lts, &v("2.10.0")));
        assert!(line_claims_version(&lts, &v("2.10.5")));
        assert!(!line_claims_version(&lts, &v("2.11.0")));
        assert!(!line_claims_version(&lts, &v("3.0.0")));
    }

    #[test]
    fn line_claims_staging() {
        let staging = Line::Staging { major: 3 };
        assert!(line_claims_version(&staging, &v("3.0.0")));
        assert!(line_claims_version(&staging, &v("3.5.0")));
        assert!(!line_claims_version(&staging, &v("2.14.0")));
    }
}
