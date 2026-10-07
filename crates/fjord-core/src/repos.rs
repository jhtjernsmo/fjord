//! Links between projects and git repositories, and each task's branch.
//! Git and GitHub access itself lives in the `fjord-vcs` crate.

use rusqlite::{OptionalExtension, params};

use crate::error::{Error, Result};
use crate::models::{ProjectRepo, RemoteHost, Task};
use crate::store::{Store, require_text};

impl Store {
    /// Links (or re-links) a project to a repository working tree.
    pub fn set_project_repo(
        &mut self,
        project_id: i64,
        path: &str,
        host: Option<&RemoteHost>,
    ) -> Result<ProjectRepo> {
        let path = require_text(path, "repository path")?;
        let project = self.get_project(project_id)?;
        let (gh_owner, gh_repo, az_org, az_project, az_repo) = match host {
            Some(RemoteHost::GitHub { owner, repo }) => (Some(owner), Some(repo), None, None, None),
            Some(RemoteHost::AzureDevOps { org, project, repo }) => {
                (None, None, Some(org), Some(project), Some(repo))
            }
            None => (None, None, None, None, None),
        };
        self.conn.execute(
            "INSERT INTO project_repos
                 (project_id, path, github_owner, github_repo, azure_org, azure_project, azure_repo)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(project_id) DO UPDATE SET path = ?2, github_owner = ?3, github_repo = ?4,
                 azure_org = ?5, azure_project = ?6, azure_repo = ?7",
            params![
                project_id, path, gh_owner, gh_repo, az_org, az_project, az_repo
            ],
        )?;
        self.log(
            Some(project_id),
            None,
            "repo.link",
            &project.name,
            Some(&path),
        )?;
        self.get_project_repo(project_id)?
            .ok_or_else(|| Error::NotFound(format!("repo for project {project_id}")))
    }

    pub fn get_project_repo(&self, project_id: i64) -> Result<Option<ProjectRepo>> {
        Ok(self
            .conn
            .query_row(
                "SELECT project_id, path, github_owner, github_repo, auto_move,
                        azure_org, azure_project, azure_repo
                 FROM project_repos WHERE project_id = ?1",
                [project_id],
                |row| {
                    Ok(ProjectRepo {
                        project_id: row.get(0)?,
                        path: row.get(1)?,
                        github_owner: row.get(2)?,
                        github_repo: row.get(3)?,
                        auto_move: row.get(4)?,
                        azure_org: row.get(5)?,
                        azure_project: row.get(6)?,
                        azure_repo: row.get(7)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn set_repo_auto_move(&mut self, project_id: i64, auto_move: bool) -> Result<()> {
        let changed = self.conn.execute(
            "UPDATE project_repos SET auto_move = ?1 WHERE project_id = ?2",
            params![auto_move, project_id],
        )?;
        if changed == 0 {
            return Err(Error::NotFound(format!("repo for project {project_id}")));
        }
        Ok(())
    }

    /// Forgets the link; the repository on disk is untouched.
    pub fn unlink_project_repo(&mut self, project_id: i64) -> Result<()> {
        let project = self.get_project(project_id)?;
        self.conn.execute(
            "DELETE FROM project_repos WHERE project_id = ?1",
            [project_id],
        )?;
        self.log(Some(project_id), None, "repo.unlink", &project.name, None)?;
        Ok(())
    }

    /// Records (or clears) the git branch a task is being worked on in.
    pub fn set_task_branch(&mut self, task_id: i64, branch: Option<&str>) -> Result<Task> {
        let task = self.get_task(task_id)?;
        let branch = branch.map(str::trim).filter(|b| !b.is_empty());
        self.conn.execute(
            "UPDATE tasks SET branch = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?2",
            params![branch, task_id],
        )?;
        if let Some(b) = branch {
            self.log(
                Some(task.project_id),
                Some(task_id),
                "task.branch",
                &task.title,
                Some(b),
            )?;
        }
        self.get_task(task_id)
    }

    /// The active task working in `branch`, if any.
    pub fn find_task_by_branch(&self, project_id: i64, branch: &str) -> Result<Option<Task>> {
        Ok(self
            .list_tasks(project_id)?
            .into_iter()
            .find(|t| t.branch.as_deref() == Some(branch)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NewProject, NewTask};
    use crate::test_util::store;

    fn setup(s: &mut Store) -> (i64, Task) {
        let p = s
            .create_project(NewProject {
                name: "P".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        let t = s
            .create_task(NewTask {
                project_id: p,
                title: "Fix push".into(),
                ..Default::default()
            })
            .unwrap();
        (p, t)
    }

    #[test]
    fn link_relink_and_unlink_repo() {
        let (mut s, _dir) = store();
        let (p, _) = setup(&mut s);
        assert!(s.get_project_repo(p).unwrap().is_none());
        let r = s
            .set_project_repo(
                p,
                "/code/bokost",
                Some(&RemoteHost::GitHub {
                    owner: "jhtjernsmo".into(),
                    repo: "bokost".into(),
                }),
            )
            .unwrap();
        assert_eq!(
            (r.github_owner.as_deref(), r.auto_move),
            (Some("jhtjernsmo"), true)
        );
        let azure = RemoteHost::AzureDevOps {
            org: "contoso".into(),
            project: "Mobile App".into(),
            repo: "bokost".into(),
        };
        let r = s.set_project_repo(p, "/code/bokost", Some(&azure)).unwrap();
        assert_eq!(
            (
                r.github_owner.as_deref(),
                r.azure_org.as_deref(),
                r.azure_project.as_deref()
            ),
            (None, Some("contoso"), Some("Mobile App")),
            "relinking to Azure clears the GitHub remote"
        );
        let r = s.set_project_repo(p, "/code/other", None).unwrap();
        assert_eq!(
            (r.path.as_str(), r.github_repo, r.azure_repo),
            ("/code/other", None, None)
        );
        s.set_repo_auto_move(p, false).unwrap();
        assert!(!s.get_project_repo(p).unwrap().unwrap().auto_move);
        s.unlink_project_repo(p).unwrap();
        assert!(s.get_project_repo(p).unwrap().is_none());
        assert!(matches!(
            s.set_repo_auto_move(p, true),
            Err(Error::NotFound(_))
        ));
        assert!(matches!(
            s.set_project_repo(p, "  ", None),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn task_branch_is_stored_found_and_cleared() {
        let (mut s, _dir) = store();
        let (p, t) = setup(&mut s);
        assert_eq!(t.branch, None);
        let t = s.set_task_branch(t.id, Some("fjord/1-fix-push")).unwrap();
        assert_eq!(t.branch.as_deref(), Some("fjord/1-fix-push"));
        assert_eq!(
            s.find_task_by_branch(p, "fjord/1-fix-push")
                .unwrap()
                .unwrap()
                .id,
            t.id
        );
        assert!(s.find_task_by_branch(p, "main").unwrap().is_none());
        assert_eq!(s.set_task_branch(t.id, Some("  ")).unwrap().branch, None);
        assert_eq!(
            s.recent_activity(Some(p), 1).unwrap()[0].action,
            "task.branch"
        );
    }
}
