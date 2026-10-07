//! Thin wrapper around the `git` command line. Using the CLI (instead of a
//! library) means Fjord sees exactly what the user sees, with their config,
//! hooks and credentials.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::{Result, VcsError};

const FIELD_SEP: char = '\u{1f}';
const MAX_BRANCH_SLUG: usize = 40;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Branch {
    pub name: String,
    pub sha: String,
    pub current: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Commit {
    pub sha: String,
    pub author: String,
    pub date: String,
    pub subject: String,
}

#[derive(Debug, Clone)]
pub struct Git {
    root: PathBuf,
}

impl Git {
    /// Opens the repository containing `path` (any folder inside it works).
    pub fn open(path: &Path) -> Result<Self> {
        let out = run(path, &["rev-parse", "--show-toplevel"])
            .map_err(|_| VcsError::NotARepo(path.display().to_string()))?;
        Ok(Self {
            root: PathBuf::from(out.trim()),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn git(&self, args: &[&str]) -> Result<String> {
        run(&self.root, args)
    }

    pub fn remote_url(&self, remote: &str) -> Option<String> {
        self.git(&["remote", "get-url", remote])
            .ok()
            .map(|s| s.trim().to_string())
    }

    pub fn current_branch(&self) -> Result<String> {
        Ok(self.git(&["branch", "--show-current"])?.trim().to_string())
    }

    pub fn branches(&self) -> Result<Vec<Branch>> {
        let current = self.current_branch().unwrap_or_default();
        let format = "--format=%(refname:short)\u{1f}%(objectname:short)\u{1f}%(upstream:short)\u{1f}%(upstream:track)";
        let out = self.git(&[
            "for-each-ref",
            format,
            "--sort=-committerdate",
            "refs/heads",
        ])?;
        Ok(out
            .lines()
            .filter_map(|line| parse_branch(line, &current))
            .collect())
    }

    pub fn commits(&self, limit: usize) -> Result<Vec<Commit>> {
        let n = format!("-n{limit}");
        let out = match self.git(&["log", &n, "--format=%h\u{1f}%an\u{1f}%aI\u{1f}%s"]) {
            Ok(out) => out,
            // A fresh repo with no commits yet.
            Err(VcsError::Git(msg)) if msg.contains("does not have any commits") => String::new(),
            Err(e) => return Err(e),
        };
        Ok(out.lines().filter_map(parse_commit).collect())
    }

    pub fn branch_exists(&self, name: &str) -> bool {
        self.git(&[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{name}"),
        ])
        .is_ok()
    }

    /// Checks out `name`, creating it from the current HEAD if it doesn't exist.
    /// Uncommitted changes come along, as with `git switch`.
    pub fn switch_or_create(&self, name: &str) -> Result<bool> {
        validate_branch_name(name)?;
        if self.branch_exists(name) {
            self.git(&["switch", name])?;
            Ok(false)
        } else {
            self.git(&["switch", "-c", name])?;
            Ok(true)
        }
    }

    pub fn push_upstream(&self, name: &str) -> Result<()> {
        validate_branch_name(name)?;
        self.git(&["push", "--set-upstream", "origin", name])?;
        Ok(())
    }

    pub fn has_uncommitted_changes(&self) -> Result<bool> {
        Ok(!self.git(&["status", "--porcelain"])?.trim().is_empty())
    }
}

fn run(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0") // never hang waiting for a password
        .output()
        .map_err(|e| VcsError::GitMissing(e.to_string()))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(VcsError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

/// Rejects names git would refuse or that could be read as options.
pub fn validate_branch_name(name: &str) -> Result<()> {
    let bad = name.is_empty()
        || name.starts_with('-')
        || name.starts_with('/')
        || name.ends_with('/')
        || name.ends_with(".lock")
        || name.contains("..")
        || name.contains("//")
        || name
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || "~^:?*[\\".contains(c));
    if bad {
        Err(VcsError::InvalidBranch(name.to_string()))
    } else {
        Ok(())
    }
}

/// "fjord/12-fix-push-notifications" for task 12 "Fix push notifications!".
pub fn branch_name_for_task(task_id: i64, title: &str) -> String {
    let slug = fjord_core::slugify(title);
    let mut slug: String = slug.chars().take(MAX_BRANCH_SLUG).collect();
    while slug.ends_with('-') {
        slug.pop();
    }
    format!("fjord/{task_id}-{slug}")
}

/// Task id encoded in a Fjord branch name, e.g. 12 for "fjord/12-fix-push".
pub fn task_id_from_branch(branch: &str) -> Option<i64> {
    let rest = branch.strip_prefix("fjord/")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

fn parse_track(track: &str) -> (u32, u32) {
    // "[ahead 2, behind 1]", "[ahead 3]", "[gone]" or ""
    let num = |key: &str| {
        track
            .split(key)
            .nth(1)
            .and_then(|r| r.trim_start().split(|c: char| !c.is_ascii_digit()).next())
            .and_then(|n| n.parse().ok())
            .unwrap_or(0)
    };
    (num("ahead"), num("behind"))
}

fn parse_branch(line: &str, current: &str) -> Option<Branch> {
    let mut parts = line.split(FIELD_SEP);
    let name = parts.next()?.to_string();
    let sha = parts.next()?.to_string();
    let upstream = parts.next().filter(|u| !u.is_empty()).map(str::to_string);
    let (ahead, behind) = parse_track(parts.next().unwrap_or(""));
    Some(Branch {
        current: name == current,
        name,
        sha,
        upstream,
        ahead,
        behind,
    })
}

fn parse_commit(line: &str) -> Option<Commit> {
    let mut parts = line.splitn(4, FIELD_SEP);
    Some(Commit {
        sha: parts.next()?.to_string(),
        author: parts.next()?.to_string(),
        date: parts.next()?.to_string(),
        subject: parts.next()?.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_names_from_tasks() {
        assert_eq!(
            branch_name_for_task(12, "Fix push notifications!"),
            "fjord/12-fix-push-notifications"
        );
        assert_eq!(
            branch_name_for_task(3, "Ærlig talt: å/ø"),
            "fjord/3-erlig-talt-a-o"
        );
        let long = branch_name_for_task(1, &"word ".repeat(30));
        assert!(long.len() <= "fjord/1-".len() + MAX_BRANCH_SLUG && !long.ends_with('-'));
        assert_eq!(task_id_from_branch("fjord/12-fix-push"), Some(12));
        assert_eq!(task_id_from_branch("feature/12-x"), None);
        assert_eq!(task_id_from_branch("fjord/abc"), None);
    }

    #[test]
    fn branch_name_validation() {
        for ok in ["main", "fjord/12-fix", "feat/a.b"] {
            assert!(validate_branch_name(ok).is_ok(), "{ok}");
        }
        for bad in ["", "-rf", "a..b", "has space", "x.lock", "a~1", "/x", "x/"] {
            assert!(validate_branch_name(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn parses_for_each_ref_and_log_output() {
        let b = parse_branch(
            "feat\u{1f}abc123\u{1f}origin/feat\u{1f}[ahead 2, behind 1]",
            "feat",
        )
        .unwrap();
        assert_eq!(
            (b.current, b.ahead, b.behind, b.upstream.as_deref()),
            (true, 2, 1, Some("origin/feat"))
        );
        let local = parse_branch("wip\u{1f}def456\u{1f}\u{1f}", "main").unwrap();
        assert_eq!(
            (local.current, local.upstream, local.ahead),
            (false, None, 0)
        );
        assert_eq!(parse_track("[gone]"), (0, 0));
        assert_eq!(parse_track("[behind 7]"), (0, 7));
        let c =
            parse_commit("a1b2c3\u{1f}Jonas\u{1f}2026-10-07T10:00:00+02:00\u{1f}feat: x \u{1f} y")
                .unwrap();
        assert_eq!(
            (c.sha.as_str(), c.subject.as_str()),
            ("a1b2c3", "feat: x \u{1f} y")
        );
    }

    fn git_in(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    /// A throwaway repo with one commit on `main`.
    pub(crate) fn temp_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        git_in(d, &["init", "-q", "-b", "main"]);
        git_in(d, &["config", "user.email", "test@example.com"]);
        git_in(d, &["config", "user.name", "Test"]);
        git_in(d, &["config", "commit.gpgsign", "false"]);
        std::fs::write(d.join("README.md"), "hi").unwrap();
        git_in(d, &["add", "."]);
        git_in(d, &["commit", "-q", "-m", "first commit"]);
        dir
    }

    #[test]
    fn works_against_a_real_repository() {
        let dir = temp_repo();
        let sub = dir.path().join("src");
        std::fs::create_dir(&sub).unwrap();
        let git = Git::open(&sub).unwrap();
        assert_eq!(git.current_branch().unwrap(), "main");
        assert!(!git.has_uncommitted_changes().unwrap());

        assert!(git.switch_or_create("fjord/1-test").unwrap());
        assert_eq!(git.current_branch().unwrap(), "fjord/1-test");
        assert!(!git.switch_or_create("main").unwrap());
        assert!(git.branch_exists("fjord/1-test"));

        let names: Vec<_> = git
            .branches()
            .unwrap()
            .into_iter()
            .map(|b| (b.name, b.current))
            .collect();
        assert!(names.contains(&("main".to_string(), true)));
        assert!(names.contains(&("fjord/1-test".to_string(), false)));
        assert_eq!(git.commits(5).unwrap()[0].subject, "first commit");
        assert_eq!(git.remote_url("origin"), None);

        std::fs::write(dir.path().join("new.txt"), "x").unwrap();
        assert!(git.has_uncommitted_changes().unwrap());
        assert!(matches!(
            git.switch_or_create("bad name"),
            Err(VcsError::InvalidBranch(_))
        ));
    }

    #[test]
    fn non_repo_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(Git::open(dir.path()), Err(VcsError::NotARepo(_))));
    }
}
