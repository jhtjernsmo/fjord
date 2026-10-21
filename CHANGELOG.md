# Changelog

All notable changes to Fjord. Versions follow [Semantic Versioning](https://semver.org); while
Fjord is below 1.0, minor versions can change behaviour. Downloads are on the
[releases page](https://github.com/jhtjernsmo/fjord/releases).

## 0.2.29 — 2026-10-21

- **Pending tasks:** mark a task as waiting on someone else, such as a reply from the client, with `w`, right-click → *Mark pending*, or the task panel. It stays in its column with a “pending 3d” badge, the board filter has a *Pending* toggle, the Overview lists pending tasks with the longest waiting first, and the standup lists them as blockers. Tasks from Azure Boards follow their work item both ways: a work item set to *Pending* in Azure becomes pending in Fjord, and marking it pending in Fjord moves the work item to *Pending* (and back to its previous state when you clear it). The state name is in **Settings → Integrations**. From the CLI: `fjord task pending <id>` and `--off`; agents use the new MCP tool `set_task_pending`.

## 0.2.28 — 2026-10-20

- **Plainer look:** smaller corners that follow the theme everywhere, a muted fjord-blue accent in Fjord Dark and Fjord Light, calmer project and column colours, no glow on buttons, and plain overdue counts in the sidebar. Nord and Solarized Light get the smaller corners too; custom themes keep their own.
- Long task titles in the standup no longer push the overview sideways; they are shortened instead.

## 0.2.27 — 2026-10-14

- **Make a task a subtask of another, or a subtask a regular task again:** right-click a task → *Make subtask of…* (pick the parent, searchable), or *Make a regular task* on a subtask. Both are in the task panel too, with `N`. From the CLI: `fjord task nest <id> <parent>` and `fjord task unnest <id>`; agents pass `parent` to `update_task`. The Azure Boards import leaves a task where you put it if Azure has no parent for it.
- **Agent Skill for Claude Code and Cursor:** `fjord skill install` adds a guide that teaches AI agents how to work in Fjord through the MCP server: check the board first, keep your text when editing, plan work, go from branch to pull request, analyse stories and triage Azure imports.
- **Agents can read a single task:** the new MCP tool `get_task` returns a task with its description, subtasks and files.
- The MCP server tells agents more about how Fjord works when they connect.

## 0.2.26 — 2026-10-13

- **Fixed: the pop-out task window stayed blank and couldn't be closed on Windows.** It now opens centred and inside the screen.

## 0.2.25 — 2026-10-13

- **Pop a task out into its own window:** the new button in the task panel (or `o`) opens the task in a window you can put beside the board or full screen on another monitor. It stays up to date, takes dropped files, and links in it open in the main window.
- **Resize the task panel:** drag its left edge; Fjord remembers the width, and a double-click resets it.
- The task panel's buttons no longer get pushed off the edge; a long name is shortened instead, and *Archive* is an icon.

## 0.2.24 — 2026-10-13

- **Move a task to another project:** right-click a task → *Move to project…*, the ⇄ button in the task panel, or `m`. Subtasks, files and the story analysis go with it, and it lands in the column with the same name (or pick one). From the CLI: `fjord task move <id> --project <name>`; agents can pass `project` to `move_task`.

## 0.2.23 — 2026-10-13

- **Get dismissed mentions back:** *Import mentions* fetches your Azure DevOps mentions again and shows the ones you dismissed or added as a task (Settings → Integrations, `Space I`, or Ctrl+K). The Overview also offers *Show hidden* when some are hidden.

## 0.2.22 — 2026-10-13

- **Acceptance criteria and risk in your analysis template:** `{{acceptance}}` and `{{risk}}` take those sections out of the task description and put them where your template says. Turn it off under Settings → Templates.
- **Risk from Azure Boards:** imported work items now bring their risk field along as a *Risk description* section. Fjord finds a field with "risk" in its name, or you name it under Settings → Integrations (e.g. `Custom.RiskDescription`).
- **Screenshots and attachments from Azure Boards:** the import downloads a work item's attachments and the images pasted into it, once each, and attaches them to the task. Pasted images show inline in the task's description. Turn it off under Settings → Integrations.
- **Send an analysis to Azure (beta, off by default):** *Send to Azure* on the analysis of an imported task writes its acceptance criteria and risk to the work item. You see each field as it is in Azure and as it will be, and pick what to send. If someone changed the work item after the preview, nothing is overwritten. Turn it on under Settings → Integrations; it needs permission to edit work items.
- **Reply in Azure discussions (beta):** with the same setting on, an imported task's Azure DevOps discussion gets a reply box (Ctrl+Enter sends). The reply is posted as you, in markdown.
- **Faster standup after start:** your tasks show at once; pull requests and mentions fill in when they arrive, and are now fetched at the same time instead of one after another. Fjord also reuses the sign-in from `az` and `gh` for a few minutes instead of starting them for every request, which made everything that talks to Azure DevOps slow for `az login` users.
- **Send to Azure shows the work item's ID and title**, and always targets the task the analysis was made for, even if the note links other imported tasks.
- **`analyze_story` gives the agent the task's context** (description, acceptance criteria, risk, work item link), so it can fill in your template for a specific task.

## 0.2.21 — 2026-10-13

- **User story analysis:** *Analyse* on a task (button, right-click, `u`, or `fjord task analyze <id>`) makes a note from your template in the project's *Analysis* folder, linked to the task and to its Azure work item. Running it again opens the same note. Edit the template under Settings → Templates.
- **PDF preview for notes:** see any note as an A4 page and save it as a PDF (the button next to the lock, or `X`).
- **`analyze_story` MCP tool**, so an agent can start an analysis and fill it in.
- **Subtasks in the standup:** work on a subtask now shows up under its parent task instead of as a line of its own, with how many of its subtasks are done. Click the arrow to see which subtasks you worked on.

## 0.2.20 — 2026-10-12

- **Git tab:** search branches, and see eight at a time with *Show all* (the branch you're on always stays in view).
- **Notes in folders and subfolders:** folders are paths like `Work/Meetings`, shown as a tree you can fold. Drag a note onto a folder to move it, double-click a folder to rename it, and sort by last changed, name or created.
- **Lock a note to the preview:** a locked note always opens read-only until you unlock it (lock button, `L`, or right-click).
- **`fjord notes`** lists, shows, moves, pins and locks notes and renames folders from the command line, so scripts and AI agents can sort your notes.
- The parent line on an agent's subtask no longer runs under the AI tag.

## 0.2.19 — 2026-10-12

- **Git tab:** shows the five pull requests that need you most, ready to merge, changes requested, failing checks or waiting for review first, with *Show all* for the rest, so branches and commits are no longer far down. Each open PR says what it needs, using review state from GitHub and Azure DevOps.

## 0.2.18 — 2026-10-12

- **Standup (beta):** on the days you have standup, the Overview drafts it for one project: what you finished and worked on since the last standup, your pull requests and where you were mentioned in Azure DevOps and GitHub, and what's in progress today. Copy it as text with one click. Turn it on and pick the days and project in **Settings → Overview**.
- **Waiting on you (beta):** pull requests waiting for your review, your own with changes requested, and the ones whose checks failed, from GitHub and Azure DevOps.
- **Overview settings:** turn each Overview section on or off.
- **Fjord for VS Code:** a new [VS Code extension](https://github.com/jhtjernsmo/fjord-vscode) shows your board next to your code, the task for the current branch in the status bar, and starts branches, moves tasks and opens pull requests.
- **`fjord git which`** prints the project and task the current folder belongs to (with `--json` for scripts and editor extensions).
- **`fjord azure import`** runs the Azure Boards import from the command line, with the same mappings and login as the app, and **`fjord azure links <project>`** lists the work items behind imported tasks.

## 0.2.17 — 2026-10-12

- A few small things for the curious. Don't panic.

## 0.2.16 — 2026-10-11

- **Calmer Overview:** today's date and a one-line summary replace the greeting and the four number cards, section headings are plain, and projects are a compact list in the same style as Due this week.
- **Quieter look:** task numbers on board cards, due dates and subtask counts as plain text instead of badges, a thin border instead of a glow on the selected card, and headings in normal case everywhere.
- **Collapsible sidebar:** `Ctrl+B` or the button next to the logo collapses the sidebar to icons, like in VS Code. Names show as tooltips, overdue projects get a red dot, and the choice is remembered.

## 0.2.15 — 2026-10-09

- **Due this week** on the Overview: open tasks from every project that are due by Sunday, overdue ones first, then by day and priority. Click one to open it.
- **Keyboard on the Overview:** `j` / `k` move through due tasks, mentions and project cards, `Enter` opens, `a` adds a mention as a task, `x` dismisses it.
- **Create GitHub repository** (`Space G`, or from the command palette) opens the create form for the current project.
- **Edit the branch name** before starting a branch: the suggested name in the task panel is now a text field. `Esc` goes back to the suggestion. ([#77](https://github.com/jhtjernsmo/fjord/pull/77))

## 0.2.14 — 2026-10-09

- The Microsoft Store package builds again (the manifest asked for a tile logo that wasn't there). ([#73](https://github.com/jhtjernsmo/fjord/pull/73))

## 0.2.13 — 2026-10-09

- Groundwork for the Microsoft Store ("Fjord Projects"): releases also build an MSIX package, and the Store version leaves updates to the Store. Nothing changes for other installs. ([#70](https://github.com/jhtjernsmo/fjord/pull/70))

## 0.2.12 — 2026-10-09

- **Azure mentions in the Overview**: work items where someone @mentioned you but that aren't assigned to you, with who mentioned you and what they wrote. Open one, add it to a project as a task, or dismiss it; new mentions get a notification. ([#66](https://github.com/jhtjernsmo/fjord/pull/66))
- The window fits the screen it opens on, also with a second monitor attached. ([#68](https://github.com/jhtjernsmo/fjord/pull/68), fixes [#65](https://github.com/jhtjernsmo/fjord/issues/65))
- Install with Homebrew: `brew install --cask jhtjernsmo/fjord/fjord` (and `fjord-cli`). ([#67](https://github.com/jhtjernsmo/fjord/pull/67))

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
