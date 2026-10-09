//! Thin wrapper around the `git` command line. Using the CLI (instead of a
//! library) means Fjord sees exactly what the user sees, with their config,
//! hooks and credentials.

use std::path::{Path, PathBuf};

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

    /// Branch names on `origin` (without the `origin/` prefix).
    pub fn remote_branches(&self) -> Result<Vec<String>> {
        let out = self.git(&[
            "for-each-ref",
            "--format=%(refname:short)",
            "--sort=-committerdate",
            "refs/remotes/origin",
        ])?;
        Ok(out
            .lines()
            .filter_map(|l| l.strip_prefix("origin/"))
            .filter(|b| *b != "HEAD" && !b.is_empty())
            .map(str::to_string)
            .collect())
    }

    /// Local branches first (most recent first), then branches only on `origin`.
    pub fn branch_names(&self) -> Result<Vec<String>> {
        let mut names: Vec<String> = self.branches()?.into_iter().map(|b| b.name).collect();
        for remote in self.remote_branches().unwrap_or_default() {
            if !names.contains(&remote) {
                names.push(remote);
            }
        }
        Ok(names)
    }

    /// The repository's main branch: what `origin/HEAD` points at, else `main`, else `master`.
    pub fn default_branch(&self) -> Result<String> {
        if let Ok(head) = self.git(&[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ]) && let Some(name) = head.trim().strip_prefix("origin/")
        {
            return Ok(name.to_string());
        }
        let remotes = self.remote_branches().unwrap_or_default();
        ["main", "master"]
            .into_iter()
            .find(|name| self.branch_exists(name) || remotes.iter().any(|r| r == name))
            .map(str::to_string)
            .ok_or_else(|| {
                VcsError::Git(
                    "couldn't find the main branch (no origin/HEAD, main or master)".into(),
                )
            })
    }

    /// Checks out `name`, creating it from the current HEAD if it doesn't exist.
    /// Uncommitted changes come along, as with `git switch`.
    pub fn switch_or_create(&self, name: &str) -> Result<bool> {
        validate_branch_name(name)?;
        if self.branch_exists(name) {
            self.git(&["switch", name])?;
            Ok(false)
        } else if self
            .remote_branches()
            .unwrap_or_default()
            .iter()
            .any(|b| b == name)
        {
            // Only on the remote: check out a local branch that tracks it.
            self.git(&["switch", "--track", &format!("origin/{name}")])?;
            Ok(false)
        } else {
            self.git(&["switch", "-c", name])?;
            Ok(true)
        }
    }

    /// False for a freshly initialised repository with no commits yet.
    pub fn has_commits(&self) -> bool {
        self.git(&["rev-parse", "--verify", "--quiet", "HEAD"])
            .is_ok()
    }

    pub fn add_remote(&self, name: &str, url: &str) -> Result<()> {
        self.git(&["remote", "add", "--", name, url])?;
        Ok(())
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

fn command(dir: &Path, args: &[&str]) -> std::process::Command {
    let mut cmd = crate::process::tool("git");
    cmd.arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0"); // never hang waiting for a password
    cmd
}

fn run(dir: &Path, args: &[&str]) -> Result<String> {
    finish(command(dir, args))
}

/// Like `run`, but lets git sign in to github.com with `token`. It goes in the
/// environment as a one-off config value, so it never shows up in the process
/// list or gets written to the repository's config.
pub(crate) fn run_with_github_token(dir: &Path, args: &[&str], token: &str) -> Result<String> {
    use base64::Engine;
    let basic = base64::engine::general_purpose::STANDARD.encode(format!("x-access-token:{token}"));
    let mut cmd = command(dir, args);
    cmd.env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "http.https://github.com/.extraheader")
        .env(
            "GIT_CONFIG_VALUE_0",
            format!("Authorization: Basic {basic}"),
        );
    finish(cmd)
}

fn finish(mut cmd: std::process::Command) -> Result<String> {
    let output = cmd
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
/// Conventional branch types, offered when starting a branch (`feat/12-add-login`).
pub const BRANCH_TYPES: &[&str] = &[
    "feat", "fix", "chore", "docs", "refactor", "test", "perf", "ci", "hotfix",
];

/// Words in a task that suggest a bug fix rather than a feature.
const FIX_WORDS: &[&str] = &[
    "fix", "bug", "crash", "error", "broken", "feil", "krasj", "fiks",
];

/// `<type>/<id>-<slug>`, e.g. "feat/12-add-login".
pub fn branch_name_for_task(kind: &str, task_id: i64, title: &str) -> String {
    let slug = fjord_core::slugify(title);
    let mut slug: String = slug.chars().take(MAX_BRANCH_SLUG).collect();
    while slug.ends_with('-') {
        slug.pop();
    }
    format!("{kind}/{task_id}-{slug}")
}

/// "fix" for tasks that read like a bug (or are a Bug in Azure Boards), else "feat".
pub fn guess_branch_type(title: &str, body: &str) -> &'static str {
    let words = title.to_lowercase();
    let is_fix = FIX_WORDS.iter().any(|w| {
        words
            .split(|c: char| !c.is_alphanumeric())
            .any(|word| word.starts_with(w))
    }) || body.contains("Azure DevOps: [Bug ");
    if is_fix { "fix" } else { "feat" }
}

/// The conventional type a branch starts with ("feat" for "feat/12-x").
pub fn branch_type(branch: &str) -> Option<&'static str> {
    let prefix = branch.split_once('/')?.0;
    BRANCH_TYPES.iter().copied().find(|t| *t == prefix)
}

/// Task id encoded in a branch Fjord created, e.g. 12 for "feat/12-add-login"
/// (and the older "fjord/12-fix-push").
pub fn task_id_from_branch(branch: &str) -> Option<i64> {
    let (prefix, rest) = branch.split_once('/')?;
    if prefix != "fjord" && !BRANCH_TYPES.contains(&prefix) {
        return None;
    }
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || !rest[digits.len()..].starts_with('-') && rest.len() != digits.len() {
        return None;
    }
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
    use std::process::Command;

    #[test]
    fn branch_names_from_tasks() {
        assert_eq!(
            branch_name_for_task("fix", 12, "Fix push notifications!"),
            "fix/12-fix-push-notifications"
        );
        assert_eq!(
            branch_name_for_task("feat", 3, "Ærlig talt: å/ø"),
            "feat/3-erlig-talt-a-o"
        );
        let long = branch_name_for_task("feat", 1, &"word ".repeat(30));
        assert!(long.len() <= "feat/1-".len() + MAX_BRANCH_SLUG && !long.ends_with('-'));
        // New conventional names and the older fjord/ ones both map back to the task.
        assert_eq!(task_id_from_branch("feat/12-add-login"), Some(12));
        assert_eq!(task_id_from_branch("fjord/12-fix-push"), Some(12));
        assert_eq!(task_id_from_branch("feature/12-x"), None);
        assert_eq!(task_id_from_branch("fix/abc"), None);
        assert_eq!(task_id_from_branch("feat/2024-roadmap"), Some(2024));
        assert_eq!(branch_type("hotfix/3-x"), Some("hotfix"));
        assert_eq!(branch_type("jonas/x"), None);
    }

    #[test]
    fn guesses_the_branch_type_from_the_task() {
        assert_eq!(guess_branch_type("Fix duplicate push", ""), "fix");
        assert_eq!(guess_branch_type("App crashes on login", ""), "fix");
        assert_eq!(guess_branch_type("Feil i budsjettet", ""), "fix");
        assert_eq!(guess_branch_type("Home screen widget", ""), "feat");
        assert_eq!(
            guess_branch_type("Prefix routes", ""),
            "feat",
            "only whole words count"
        );
        assert_eq!(
            guess_branch_type("Login", "…\n---\nAzure DevOps: [Bug 297](…)"),
            "fix"
        );
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
