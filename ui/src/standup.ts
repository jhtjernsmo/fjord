// Standup drafts: which weekdays have a standup, and what happened since the last one.
import { useCallback } from 'react'
import type { Activity, Status, Task } from './api'
import { useStoredPref } from './storedPref'

/** Weekdays as Date.getDay() numbers: 0 is Sunday, 1 Monday … 6 Saturday. */
export type Weekday = 0 | 1 | 2 | 3 | 4 | 5 | 6

/** Monday first, the way calendars here are laid out. */
export const WEEK: Weekday[] = [1, 2, 3, 4, 5, 6, 0]

const DAYS_KEY = 'fjord.standupDays'

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

export interface StandupDraft {
  done: Task[]
  workedOn: Task[]
  /** In progress now. What's due is already in "Due this week". */
  today: Task[]
}

/** Actions that count as working on a task. Edits to columns, notes and files don't. */
const WORK_ACTIONS = new Set(['task.create', 'task.move', 'task.update', 'task.branch'])

/**
 * Builds the draft from the activity log and the current boards.
 * Only `actor`'s own changes count, so tasks an agent moved don't end up in your standup.
 */
export function buildStandup(activity: Activity[], tasks: Task[], statuses: Status[], since: Date, actor: string): StandupDraft {
  const doneStatus = new Set(statuses.filter((s) => s.is_done).map((s) => s.id))
  const firstStatus = new Set(firstColumns(statuses))
  const byId = new Map(tasks.filter((t) => !t.archived_at).map((t) => [t.id, t]))
  const touched = new Set<number>()
  for (const a of activity) {
    if (a.actor !== actor || a.task_id === null || !WORK_ACTIONS.has(a.action)) continue
    if (new Date(a.created_at) >= since) touched.add(a.task_id)
  }
  const recent = [...touched].map((id) => byId.get(id)).filter((t): t is Task => t !== undefined)
  const done = recent.filter((t) => doneStatus.has(t.status_id))
  const workedOn = recent.filter((t) => !doneStatus.has(t.status_id))

  const inProgress = [...byId.values()].filter((t) => !doneStatus.has(t.status_id) && !firstStatus.has(t.status_id) && !t.parent_id)
  return { done, workedOn, today: inProgress }
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
}

/** Plain text for pasting into Teams or Slack. */
export function standupText(draft: StandupDraft, labels: StandupLabels): string {
  const line = (t: Task, extra = '') => `- ${t.title} (#${t.id})${extra}`
  const lines = [labels.since]
  for (const t of draft.done) lines.push(line(t, ` – ${labels.done}`))
  for (const t of draft.workedOn) lines.push(line(t))
  if (draft.done.length + draft.workedOn.length === 0) lines.push(`- ${labels.none}`)
  lines.push('', labels.today)
  for (const t of draft.today) lines.push(line(t))
  if (draft.today.length === 0) lines.push(`- ${labels.none}`)
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
