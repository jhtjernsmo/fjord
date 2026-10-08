# Azure Boards import and AI triage

Fjord can pull the Azure Boards work items assigned to you into Fjord projects,
and an AI agent can then analyze each new one through Fjord's MCP server.

## 1. Import (built into Fjord)

1. **Settings → Azure DevOps**: connect your organization with a personal access
   token. For the import it needs **Work Items (Read)** in addition to Code and Build.
2. **Settings → Azure Boards import**: add a mapping per Azure project, e.g.
   `contoso` / `Mobile App` → Fjord project *Bokost*, and turn the import on.

Fjord then checks on startup and every 10 minutes for open work items where
*Assigned To = you*. Each one becomes a task in the mapped project, with:

- title, description and acceptance criteria (HTML converted to markdown)
- priority (Azure 1/2/3/4 → high/medium/low/none) and due or target date
- a footer linking back to the work item, with its type, state and area

Re-imports never create duplicates. When the work item changes in Azure, the
title, priority and due date are refreshed. The description is left alone,
because you or an agent may have added to it.

Work items in Azure projects without a mapping are skipped.

**Hierarchy.** If a work item assigned to you has a parent (or grandparent) that is
also assigned to you, it becomes a subtask of it in Fjord, e.g. your Tasks under
your User Story. Fjord has one level of subtasks, so deeper chains (Epic → Feature →
Story → Task) all go under the topmost item assigned to you. Parents assigned to
others are looked through but not imported. Tasks already imported move to the
right place on the next import.

**Closed in Azure.** When a work item you imported is closed, done, resolved or
removed in Azure, its Fjord task is moved to your done column on the next import
(only once, so you can still move it back).

## 2. AI triage via MCP

Imported tasks stay on a "new" list until an agent marks them analyzed. The MCP
server (`fjord mcp`) has three tools for this:

| Tool | What it does |
|---|---|
| `import_azure` | Run the import now (same as the app does every 10 minutes) |
| `list_new_imports` | Imported tasks nobody has analyzed yet |
| `mark_analyzed` | Take a task off that list |

Agents can create and edit tasks but never delete them, and every change shows
up in the activity log as made by the agent.

### The prompt

Save this as `fjord-triage.md`:

```markdown
You are triaging new work in Fjord (MCP server "fjord").

1. Call import_azure to fetch the latest work items, then list_new_imports.
2. For each task returned:
   - Read the description, acceptance criteria and the Azure link footer.
   - Use update_task to append to the body (keep everything that is there) a section:
     ## Analysis
     - Summary in 1-2 sentences
     - Acceptance criteria as a checklist (- [ ] …), from Azure or inferred
     - Open questions, if anything is unclear
     - Suggested steps; if it is bigger than a day of work, create subtasks with
       create_task (parent = the task's id) in the same project
     - Rough size: S (< 2h), M (< 1 day) or L (more)
   - Set priority if the work item had none and urgency is obvious.
   - Call mark_analyzed with the task id.
3. Finish with a short summary: which tasks were analyzed and anything that needs a human.
Never delete or archive anything.
```

### Run it on a schedule

With Claude Code and Fjord's MCP server registered once
(`claude mcp add fjord -- fjord mcp`):

**Windows** (Task Scheduler, every weekday at 08:00):

```powershell
schtasks /create /tn "Fjord triage" /sc weekly /d MON,TUE,WED,THU,FRI /st 08:00 /tr "powershell -NoProfile -Command claude -p (Get-Content $env:USERPROFILE\fjord-triage.md -Raw) --allowedTools mcp__fjord"
```

**macOS / Linux** (cron, weekdays at 08:00):

```cron
0 8 * * 1-5  claude -p "$(cat ~/fjord-triage.md)" --allowedTools mcp__fjord
```

`--allowedTools mcp__fjord` lets the run use Fjord's tools without asking, and
nothing else. You can also run the same command by hand whenever you like.
