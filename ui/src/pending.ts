// Pending tasks: waiting on someone else, typically a reply from the client.
import type { Task } from './api'
import type { Translate } from './i18n'

const DAY_MS = 86_400_000

export function isPending(task: Task): boolean {
  return task.pending_since !== null && task.pending_since !== undefined
}

/** Whole days since the task started waiting; 0 on the first day. */
export function pendingDays(since: string, now: Date = new Date()): number {
  return Math.max(0, Math.floor((now.getTime() - new Date(since).getTime()) / DAY_MS))
}

/** "pending 3d" for cards; just "pending" on the first day. */
export function pendingChip(since: string, t: Translate, now?: Date): string {
  const n = pendingDays(since, now)
  return n === 0 ? t('pending.chipToday') : t('pending.chip', { n })
}

/** "3 days", "1 day" or "today", for lists. */
export function waitedFor(since: string, t: Translate, now?: Date): string {
  const n = pendingDays(since, now)
  return n === 0 ? t('pending.waitedToday') : n === 1 ? t('pending.waitedOne') : t('pending.waited', { n })
}
