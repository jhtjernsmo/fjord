//! The Fjord Agent Skill: a guide that teaches AI agents how to use the MCP tools.
//! Claude Code and Cursor both load skills from `~/.claude/skills`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};

pub const SKILL_MD: &str = include_str!("../../../skills/fjord/SKILL.md");
const SKILL_NAME: &str = "fjord";

/// `~/.claude/skills`, where Claude Code and Cursor look for user-level skills.
pub fn default_root() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("could not find your home folder"))?;
    Ok(home.join(".claude").join("skills"))
}

/// Writes the skill to `<root>/fjord/SKILL.md`, replacing an older version. Returns the file.
pub fn install(root: &Path) -> Result<PathBuf> {
    let dir = root.join(SKILL_NAME);
    fs::create_dir_all(&dir).with_context(|| format!("could not create {}", dir.display()))?;
    let file = dir.join("SKILL.md");
    fs::write(&file, SKILL_MD).with_context(|| format!("could not write {}", file.display()))?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_has_the_frontmatter_agents_need() {
        // Normalised, in case git checked the file out with CRLF line endings.
        let skill = SKILL_MD.replace("\r\n", "\n");
        let front = skill
            .strip_prefix("---\n")
            .and_then(|rest| rest.split_once("\n---\n"))
            .map(|(front, _)| front)
            .expect("frontmatter");
        assert!(front.lines().any(|l| l == "name: fjord"));
        assert!(front.lines().any(|l| l.starts_with("description: ")));
    }

    #[test]
    fn install_writes_and_replaces_the_skill() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("fjord").join("SKILL.md");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "old").unwrap();

        assert_eq!(install(root.path()).unwrap(), file);
        assert_eq!(fs::read_to_string(&file).unwrap(), SKILL_MD);
    }
}
