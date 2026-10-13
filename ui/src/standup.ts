// Standup drafts: which weekdays have a standup, and what happened since the last one.
import { useCallback } from 'react'
import type { Activity, GitHubMention, Mention, PullRequest, Status, Task } from './api'
import { useStoredPref } from './storedPref'

/** Weekdays as Date.getDay() numbers: 0 is Sunday, 1 Monday … 6 Saturday. */
export type Weekday = 0 | 1 | 2 | 3 | 4 | 5 | 6

/** Monday first, the way calendars here are laid out. */
export const WEEK: Weekday[] = [1, 2, 3, 4, 5, 6, 0]

const DAYS_KEY = 'fjord.standupDays'
const PROJECT_KEY = 'fjord.standupProject'

export function isStandupDay(date: Date, days: Weekday[]): boolean {
  return days.includes(date.getDay() as Weekday)
}

/**
 * The start of the most recent standup day before `today`, or a week back when there's only one
 * standup a week. Activity from then on goes into today's draft.
 */
export function previousStandup(today: Date, days: Weekday[]): Date {
  for (let back = 1; back <= 7; back++) {
    const day = new Date(today.getFullYear(), today.getMonth(), today.getDate() - back)
    if (isStandupDay(day, days)) return day
  }
  return new Date(today.getFullYear(), today.getMonth(), today.getDate() - 7)
}

/** A top-level task in the standup. Subtasks don't get lines of their own; they sit under their parent. */
export interface StandupEntry {
  task: Task
  /** The task's subtasks you worked on since the last standup. */
  subtasks: { task: Task; done: boolean }[]
  /** How many of all its subtasks are done; null for a task without subtasks. */
  progress: { done: number; total: number } | null
}

export interface StandupDraft {
  done: StandupEntry[]
  workedOn: StandupEntry[]
  /** In progress now. What's due is already in "Due this week". */
  today: StandupEntry[]
  /** Azure DevOps work items where someone @mentioned you since the last standup. */
  mentions: Mention[]
  /** GitHub issues and pull requests that @mention you, updated since the last standup. */
  githubMentions: GitHubMention[]
  /** Your pull requests merged or closed since the last standup. */
  closedPrs: PullRequest[]
  /** Your pull requests that are still open. */
  openPrs: PullRequest[]
}

/** Actions that count as working on a task. Edits to columns, notes and files don't. */
const WORK_ACTIONS = new Set(['task.create', 'task.move', 'task.update', 'task.branch'])

/**
 * Builds the draft from the activity log and the current boards.
 * Only `actor`'s own changes count, so tasks an agent moved don't end up in your standup.
 * Work on a subtask shows up as work on its parent, with the subtask listed under it.
 */
export function buildStandup(activity: Activity[], tasks: Task[], statuses: Status[], since: Date, actor: string): Pick<StandupDraft, 'done' | 'workedOn' | 'today'> {
  const doneStatus = new Set(statuses.filter((s) => s.is_done).map((s) => s.id))
  const firstStatus = new Set(firstColumns(statuses))
  const byId = new Map(tasks.filter((t) => !t.archived_at).map((t) => [t.id, t]))
  const touched = new Set<number>()
  for (const a of activity) {
    if (a.actor !== actor || a.task_id === null || !WORK_ACTIONS.has(a.action)) continue
    if (new Date(a.created_at) < since) continue
    touched.add(a.task_id)
  }

  const parentOf = (t: Task) => (t.parent_id === null ? undefined : byId.get(t.parent_id))
  const children = new Map<number, Task[]>()
  for (const t of byId.values()) {
    const parent = parentOf(t)
    if (parent) children.set(parent.id, [...(children.get(parent.id) ?? []), t])
  }
  // A subtask whose parent is gone or archived stands on its own.
  const tops = new Map<number, Task[]>()
  for (const id of touched) {
    const t = byId.get(id)
    if (!t) continue
    const top = parentOf(t) ?? t
    const subtasks = tops.get(top.id) ?? []
    tops.set(top.id, top === t ? subtasks : [...subtasks, t])
  }

  const isDone = (t: Task) => doneStatus.has(t.status_id)
  const entry = (task: Task, subtasks: Task[] = []): StandupEntry => {
    const all = children.get(task.id)
    return { task, subtasks: subtasks.map((t) => ({ task: t, done: isDone(t) })), progress: all ? { done: all.filter(isDone).length, total: all.length } : null }
  }
  const recent = [...tops].map(([id, subtasks]) => entry(byId.get(id) as Task, subtasks))
  const inProgress = [...byId.values()].filter((t) => !isDone(t) && !firstStatus.has(t.status_id) && !parentOf(t)).map((t) => entry(t))
  return { done: recent.filter((e) => isDone(e.task)), workedOn: recent.filter((e) => !isDone(e.task)), today: inProgress }
}

/**
 * Your own pull requests, matched on the author name GitHub or Azure DevOps reports for you.
 * Open ones belong under today; merged or closed ones under "since" if that happened after the last standup.
 */
export function myPullRequests(prs: PullRequest[], me: string[], since: Date): Pick<StandupDraft, 'closedPrs' | 'openPrs'> {
  const names = new Set(me.map((m) => m.toLowerCase()))
  const mine = prs.filter((p) => names.has(p.author.toLowerCase()))
  return {
    closedPrs: mine.filter((p) => p.state !== 'open' && new Date(p.updated_at) >= since),
    openPrs: mine.filter((p) => p.state === 'open'),
  }
}

/**
 * Mentions since the last standup. With `projects` set (Azure org and project pairs mapped to the
 * chosen Fjord project), only mentions from those projects count.
 */
export function mentionsSince(mentions: Mention[], since: Date, projects: { org: string; project: string }[] | null): Mention[] {
  const inScope = (m: Mention) => projects === null || projects.some((p) => p.org.toLowerCase() === m.org.toLowerCase() && p.project === m.project)
  return mentions.filter((m) => new Date(m.at) >= since && inScope(m)).sort((a, b) => b.at.localeCompare(a.at))
}

/** Each project's first column is its backlog; tasks there aren't "in progress". */
function firstColumns(statuses: Status[]): number[] {
  const first = new Map<number, Status>()
  for (const s of statuses) {
    const current = first.get(s.project_id)
    if (!current || s.position < current.position) first.set(s.project_id, s)
  }
  return [...first.values()].map((s) => s.id)
}

export interface StandupLabels {
  since: string
  done: string
  today: string
  blockers: string
  none: string
  merged: string
  closed: string
  inReview: string
  mentioned: string
  /** e.g. "2/5 subtasks done". */
  subtasks: (done: number, total: number) => string
}

/** Plain text for pasting into Teams or Slack. */
export function standupText(draft: StandupDraft, labels: StandupLabels): string {
  const line = ({ task, progress }: StandupEntry, extra = '') => `- ${task.title} (#${task.id})${progress ? `, ${labels.subtasks(progress.done, progress.total)}` : ''}${extra}`
  const lines = [labels.since]
  for (const e of draft.done) lines.push(line(e, ` – ${labels.done}`))
  const pr = (p: PullRequest, state: string) => `- PR #${p.number} ${p.title} – ${state}`
  for (const e of draft.workedOn) lines.push(line(e))
  for (const p of draft.closedPrs) lines.push(pr(p, p.state === 'merged' ? labels.merged : labels.closed))
  for (const m of draft.mentions) lines.push(`- ${labels.mentioned} #${m.id} ${m.title} (${m.by})`)
  for (const m of draft.githubMentions) lines.push(`- ${labels.mentioned} ${m.repo}#${m.number} ${m.title} (${m.by})`)
  if (draft.done.length + draft.workedOn.length + draft.closedPrs.length + draft.mentions.length + draft.githubMentions.length === 0) lines.push(`- ${labels.none}`)
  lines.push('', labels.today)
  for (const e of draft.today) lines.push(line(e))
  for (const p of draft.openPrs) lines.push(pr(p, labels.inReview))
  if (draft.today.length + draft.openPrs.length === 0) lines.push(`- ${labels.none}`)
  lines.push('', labels.blockers, `- ${labels.none}`)
  return lines.join('\n')
}

const NO_DAYS: Weekday[] = []

function parseDays(raw: unknown): Weekday[] {
  return Array.isArray(raw) ? raw.filter((d): d is Weekday => Number.isInteger(d) && d >= 0 && d <= 6) : []
}

/** Standup weekdays, a per-device preference like the theme. Empty means no standups. */
export function useStandupDays(): [Weekday[], (day: Weekday) => void] {
  const [days, update] = useStoredPref(DAYS_KEY, parseDays, NO_DAYS)
  const toggle = useCallback((day: Weekday) => update((d) => (d.includes(day) ? d.filter((x) => x !== day) : [...d, day])), [update])
  return [days, toggle]
}

function parseProject(raw: unknown): number | null {
  return typeof raw === 'number' && Number.isInteger(raw) ? raw : null
}

/** The project the standup is about, or null for all of them. */
export function useStandupProject(): [number | null, (id: number | null) => void] {
  const [project, update] = useStoredPref(PROJECT_KEY, parseProject, null)
  const set = useCallback((id: number | null) => update(() => id), [update])
  return [project, set]
}
