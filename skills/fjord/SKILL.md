---
name: fjord
description: Work with the user's projects, tasks and notes in Fjord, their local project manager, through the fjord MCP server. Use when the user mentions Fjord, their board, backlog, tasks, todos, notes, user stories or Azure Boards work items, or asks to plan, track, start, finish or analyse work in a project.
---

# Fjord

Fjord is the user's local project manager: projects with kanban columns, tasks with one level of
subtasks, markdown notes with `[[links]]`, attached files, and links to git repositories and Azure
Boards. Everything lives in one local database that the desktop app, the `fjord` CLI and the MCP
server share, so the user sees your changes in the app straight away, tagged as made by an agent.

## Setup

The tools come from the MCP server `fjord mcp`. If no Fjord tools are available, tell the user to
add it:

- Claude Code: `claude mcp add fjord -- fjord mcp`
- Cursor and others: `{ "mcpServers": { "fjord": { "command": "fjord", "args": ["mcp"] } } }`

The `fjord` CLI (`fjord --help`, add `--json` for machine-readable output) does the same things and
works when MCP doesn't.

## Ground rules

- **Look before you change.** Call `get_board` for a project before creating or moving tasks, so you
  use its real column names and don't create duplicates. Use `search` to find a task or note when
  you don't know its id, and `get_task` to read one task with its subtasks and files.
- **Nothing can be deleted.** There is no delete tool on purpose. To remove a task, `archive_task`
  (the user can restore it). Never claim you deleted something.
- **Projects** are referenced by id or slug (`bokost`), tasks and notes by number id. In text, write
  a task as `#12`.
- **Columns** are per project and named by the user (often To do, In progress, Done). `move_task`
  takes a column name or id; `complete_task` moves to whichever column counts as done.
- **Priority** is 0 none, 1 low, 2 medium, 3 high. Due dates are `YYYY-MM-DD`.
- **Bodies are markdown.** `update_task` with `body` replaces the whole description, so read the task
  first with `get_task` and keep what the user wrote. `update_note` has `append` for adding
  to the end without rewriting.
- **Subtasks**: `create_task` with `parent`. A subtask can't have subtasks of its own.
- Write in the language the user writes in. Keep task titles short and imperative ("Fix login on
  Safari"), and put detail in the body.
- Don't invent facts in tasks, notes or analyses. Leave a gap or ask.

## Common workflows

**Plan work into tasks.** `list_projects` → pick or `create_project` → `get_board` → `create_task`
for each piece, with subtasks under a parent when the work splits naturally. Confirm the list with
the user first if it's more than a handful of tasks.

**Pick up a task and code it.** `get_board` or `search` to find it, `get_task` to read it → if the project has a linked
repo, `start_branch` (checks out `<type>/<id>-<title>` and moves the task to in progress) → do the
work → commit → `open_pull_request` when the user wants a PR → `list_pull_requests` later shows CI
state and moves tasks with merged PRs to done. Without a linked repo, `link_repo` with the local path
first, or just `move_task` by hand.

**Report progress.** `recent_activity` (optionally per project) and `list_projects` for counts.
Summarise; don't paste raw JSON.

**Notes and knowledge.** `add_note` with `project` for project notes, without it for the global
notespace; `folder` like `product/ideas`. Link with `[[Project]]`, `[[#12]]` or `[[Note title]]`.
`list_notes` lists without bodies; `get_note` reads one with its links resolved; `backlinks` shows
what links to a project, task or note.

**Analyse a user story.** `analyze_story` with the task id creates (or returns) an analysis note from
the user's own template, with the task's context. Fill it in with one `update_note` call: keep the
user's headings, their order and the `[[#id]]` link, and leave a section's placeholder line when
there is nothing to say.

**Triage Azure Boards imports.** `import_azure` (optional; Fjord also imports on its own) →
`list_new_imports` → for each: `get_task`, then `update_task` to add a short summary, acceptance criteria
and suggested subtasks to the body (keep the original text and the work item footer),
`create_task` for subtasks when useful → `mark_analyzed`. Don't change the title or priority; they
come from Azure.

**Move a task to another project.** `move_task` with `project` (name or slug). Its subtasks, files
and analysis go with it; without `status` it lands in the column with the same name.

**Files.** `attach_file` copies a local file into Fjord and attaches it to a project or task.

## When something fails

Tool errors come back as text ("Error: …"): a wrong column name, an unknown project or task, or an
archived project. Fix the argument (check `get_board`) rather than retrying the same call. Git and
pull request tools need a linked repository and, for GitHub, the `gh` CLI signed in.
