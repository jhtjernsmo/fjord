//! Opening a linked repository in the user's editor or IDE.
//!
//! The editor is a command template such as `code {path}` or
//! `open -a "IntelliJ IDEA" {path}`. It runs directly, never through a shell,
//! so a repository path can't inject anything. On Windows, editors usually
//! install a `.cmd` launcher (`code.cmd`), which is looked up on PATH.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::{Result, VcsError};

const PATH_PLACEHOLDER: &str = "{path}";

/// Splits a command template into words. Double quotes group words and are removed.
fn split_words(template: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut in_word = false;
    for c in template.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                in_word = true;
            }
            c if c.is_whitespace() && !quoted => {
                if in_word {
                    words.push(std::mem::take(&mut current));
                    in_word = false;
                }
            }
            c => {
                current.push(c);
                in_word = true;
            }
        }
    }
    if in_word {
        words.push(current);
    }
    words
}

/// The program and arguments for opening `path`: `{path}` is replaced, or the
/// path is added at the end when the template doesn't mention it.
pub fn editor_command(template: &str, path: &Path) -> Result<(String, Vec<String>)> {
    let path = path.to_string_lossy();
    let mut words = split_words(template);
    if words.is_empty() {
        return Err(VcsError::Git(
            "no editor is set; choose one in Settings → Integrations".into(),
        ));
    }
    let mentions_path = words.iter().any(|w| w.contains(PATH_PLACEHOLDER));
    for word in words.iter_mut() {
        *word = word.replace(PATH_PLACEHOLDER, &path);
    }
    if !mentions_path {
        words.push(path.into_owned());
    }
    let program = words.remove(0);
    Ok((program, words))
}

/// On Windows, finds `program` on PATH with the usual extensions (`code` → `code.cmd`).
#[cfg(windows)]
fn resolve(program: &str) -> PathBuf {
    let direct = Path::new(program);
    if direct.components().count() > 1 || direct.extension().is_some() {
        return direct.to_path_buf();
    }
    let exts = ["exe", "cmd", "bat", "com"];
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .flat_map(|dir| {
            exts.iter()
                .map(move |ext| dir.join(format!("{program}.{ext}")))
        })
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| direct.to_path_buf())
}

#[cfg(not(windows))]
fn resolve(program: &str) -> PathBuf {
    PathBuf::from(program)
}

/// Starts the editor on `path` and returns without waiting for it.
pub fn open_in_editor(template: &str, path: &Path) -> Result<()> {
    let (program, args) = editor_command(template, path)?;
    let mut cmd = Command::new(resolve(&program));
    cmd.args(&args)
        .current_dir(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // No console window flashing up for .cmd launchers.
        cmd.creation_flags(0x0800_0000);
    }
    cmd.spawn().map(drop).map_err(|e| {
        let hint = if e.kind() == std::io::ErrorKind::NotFound {
            format!(
                "«{program}» wasn't found. Install the editor's command-line launcher, or set the full path in Settings → Integrations"
            )
        } else {
            format!("couldn't start «{program}»: {e}")
        };
        VcsError::Git(hint)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_the_editor_command() {
        let path = Path::new("/home/j/code/my repo");
        assert_eq!(
            editor_command("code", path).unwrap(),
            ("code".into(), vec!["/home/j/code/my repo".to_string()])
        );
        assert_eq!(
            editor_command(r#"open -a "IntelliJ IDEA" {path}"#, path).unwrap(),
            (
                "open".into(),
                vec![
                    "-a".to_string(),
                    "IntelliJ IDEA".into(),
                    "/home/j/code/my repo".into()
                ]
            )
        );
        assert_eq!(
            editor_command(r#""C:\Program Files\Zed\zed.exe" --new {path}"#, path)
                .unwrap()
                .0,
            r"C:\Program Files\Zed\zed.exe"
        );
        assert!(editor_command("   ", path).is_err());
    }

    #[test]
    fn a_path_cannot_add_arguments_or_commands() {
        // The path stays one argument, whatever it contains.
        let path = Path::new("/tmp/x; touch pwned & calc");
        let (_, args) = editor_command("code --reuse-window", path).unwrap();
        assert_eq!(
            args,
            vec![
                "--reuse-window".to_string(),
                "/tmp/x; touch pwned & calc".into()
            ]
        );
    }

    #[test]
    fn missing_editor_gives_a_helpful_error() {
        let err = open_in_editor("fjord-no-such-editor-xyz", Path::new("."))
            .unwrap_err()
            .to_string();
        assert!(err.contains("wasn't found"), "{err}");
    }
}
