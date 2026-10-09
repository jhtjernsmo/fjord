# Changelog

All notable changes to Fjord. Versions follow [Semantic Versioning](https://semver.org); while
Fjord is below 1.0, minor versions can change behaviour. Downloads are on the
[releases page](https://github.com/jhtjernsmo/fjord/releases).

## 0.2.11 — 2026-10-09

- **Create a GitHub repository** when you make a project, or from the Git tab: private or public, under you or an organization, with an optional README, `.gitignore` and license. Fjord clones it and links it to the project. A local repository without a remote can be published the same way. ([#57](https://github.com/jhtjernsmo/fjord/pull/57))
- The Git buttons on a task are now small icons next to the branch name, which stays on one line. ([#53](https://github.com/jhtjernsmo/fjord/pull/53))
- Weekly dependency updates with Dependabot; TypeScript 7. ([#56](https://github.com/jhtjernsmo/fjord/pull/56), [#59](https://github.com/jhtjernsmo/fjord/pull/59), [#60](https://github.com/jhtjernsmo/fjord/pull/60))
- The project has the OpenSSF Best Practices badge, a `LICENSE` file, a privacy policy and a code signing policy. ([#51](https://github.com/jhtjernsmo/fjord/pull/51), [#54](https://github.com/jhtjernsmo/fjord/pull/54))

## 0.2.10 — 2026-10-08

- **Open in editor**: open a project's repository in your editor or IDE (VS Code, Cursor, JetBrains IDEs, Visual Studio, Zed, Sublime, or your own command), on a task's branch. `Space e`. ([#48](https://github.com/jhtjernsmo/fjord/pull/48))
- **Back to main**: switch the repository back to its main branch from a task or the Git tab; `fjord git main`. ([#46](https://github.com/jhtjernsmo/fjord/pull/46))
- **Project sorting**: numbered then A–Z (default), alphabetical, last used or newest first, and reversed. ([#47](https://github.com/jhtjernsmo/fjord/pull/47))

## 0.2.9 — 2026-10-08

- The macOS app is signed with an Apple Developer ID and notarized.
- Settings → Updates shows the installed version. ([#43](https://github.com/jhtjernsmo/fjord/pull/43))

## 0.2.8 — 2026-10-08

- Conventional branch names (`feat/12-add-login`), with the type picked or guessed, and PR titles in the same style. ([#41](https://github.com/jhtjernsmo/fjord/pull/41))
- One branch can be linked to several tasks. ([#40](https://github.com/jhtjernsmo/fjord/pull/40))
- GitHub and Azure DevOps errors show the service's explanation instead of a bare status code. ([#39](https://github.com/jhtjernsmo/fjord/pull/39))
- Azure Boards import also brings in recently finished work items. ([#38](https://github.com/jhtjernsmo/fjord/pull/38))

## 0.2.7 — 2026-10-08

- Change a project's icon and colour; read an Azure work item's discussion in the task; notifications for imports; a *Report a bug* button. ([#36](https://github.com/jhtjernsmo/fjord/pull/36))
- Mermaid diagrams in notes and task descriptions. ([#34](https://github.com/jhtjernsmo/fjord/pull/34))
- More keyboard shortcuts for the task panel, notes and new features. ([#31](https://github.com/jhtjernsmo/fjord/pull/31))
- Azure states map to columns with the same name. ([#32](https://github.com/jhtjernsmo/fjord/pull/32))
- Fixed: web links open in the system browser ([#35](https://github.com/jhtjernsmo/fjord/pull/35)); dropping a card works anywhere in a tall column ([#33](https://github.com/jhtjernsmo/fjord/pull/33)); auto-moves leave tasks that are already in a done column ([#30](https://github.com/jhtjernsmo/fjord/pull/30)).

## 0.2.6 — 2026-10-08

- New app icon. ([#28](https://github.com/jhtjernsmo/fjord/pull/28))

## 0.2.5 — 2026-10-08

- Link a task to an existing branch. ([#25](https://github.com/jhtjernsmo/fjord/pull/25))
- Azure Boards import keeps the parent/child hierarchy as subtasks and checks off closed items. ([#26](https://github.com/jhtjernsmo/fjord/pull/26))

## 0.2.4 — 2026-10-08

- Subtasks. ([#22](https://github.com/jhtjernsmo/fjord/pull/22))
- A settings window with sections, and custom themes. ([#23](https://github.com/jhtjernsmo/fjord/pull/23))
- Sign in with GitHub in the browser, and Azure DevOps through `az login`. ([#19](https://github.com/jhtjernsmo/fjord/pull/19))
- Fixed: dialogs scroll on short screens ([#20](https://github.com/jhtjernsmo/fjord/pull/20)); an Azure mapping row could disappear while editing ([#21](https://github.com/jhtjernsmo/fjord/pull/21)).

## 0.2.3 — 2026-10-08

- Azure Boards import (beta), with AI triage through the MCP server. ([#16](https://github.com/jhtjernsmo/fjord/pull/16))
- Fixed: long task titles overflowing the card ([#17](https://github.com/jhtjernsmo/fjord/pull/17)); the Linux CLI build ([#15](https://github.com/jhtjernsmo/fjord/pull/15)).

## 0.2.2 — 2026-10-07

- Azure DevOps repositories (beta). ([#12](https://github.com/jhtjernsmo/fjord/pull/12))
- Private GitHub repositories, with the token kept in the system keychain. ([#10](https://github.com/jhtjernsmo/fjord/pull/10))
- A custom title bar. ([#11](https://github.com/jhtjernsmo/fjord/pull/11))
- MCP tools for notes. ([#7](https://github.com/jhtjernsmo/fjord/pull/7))
- Fixed: markdown tables. ([#14](https://github.com/jhtjernsmo/fjord/pull/14))

## 0.2.1 — 2026-10-07

- Signed in-app updates. ([#6](https://github.com/jhtjernsmo/fjord/pull/6))

## 0.2.0 — 2026-10-07

- Notespace: markdown notes with `[[links]]`, folders, pins and backlinks. ([#5](https://github.com/jhtjernsmo/fjord/pull/5))

## 0.1.1 — 2026-10-07

- App icon and fixes from the first round of feedback. ([#2](https://github.com/jhtjernsmo/fjord/pull/2), [#4](https://github.com/jhtjernsmo/fjord/pull/4))

## 0.1.0 — 2026-10-07

- First release: projects, kanban boards, tasks, files, search and activity log on SQLite; the desktop app, the `fjord` CLI and the MCP server; English and Norwegian.
