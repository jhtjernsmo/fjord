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

**Finished items.** By default the import also brings in work items assigned to you that
are already finished (Done, Resolved, Closed, …) and changed in the last 30 days, so
recent work shows up too. They land straight in the matching column (*Resolved* →
*Resolved*) or your done column. Change the window, or turn it off, under
**Settings → Integrations**.

**Hierarchy.** If a work item assigned to you has a parent (or grandparent) that is
also assigned to you, it becomes a subtask of it in Fjord, e.g. your Tasks under
your User Story. Fjord has one level of subtasks, so deeper chains (Epic → Feature →
Story → Task) all go under the topmost item assigned to you. Parents assigned to
others are looked through but not imported. Tasks already imported move to the
right place on the next import.

**Open states.** When a work item's state changes in Azure, its task follows on the
next import: to a column with the same name as the state if you have one; otherwise
*Active*, *Committed* and *In Progress* go to your *In progress* (or *Pågår*) column,
and *New* goes to the first column. Fjord only moves a task when the state actually
changed in Azure, so moving it by hand sticks.

**Closed in Azure.** When a work item you imported is closed, done, resolved or
removed in Azure, its Fjord task moves on the next import: to a column with the same
name as the Azure state if you have one (e.g. *Resolved* → your *Resolved* column,
*Done* → *Done*), otherwise to your done column. Tasks already in a done column stay
put unless a same-named column exists.
