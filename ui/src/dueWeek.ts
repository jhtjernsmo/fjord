// Date helpers for the Overview's "Due this week" list. Weeks run Monday to Sunday.

export type DueWhen = 'overdue' | 'today' | 'tomorrow' | 'later'

/** A local calendar date as YYYY-MM-DD (what due dates are stored as). */
export function localIso(date: Date): string {
  const m = String(date.getMonth() + 1).padStart(2, '0')
  const d = String(date.getDate()).padStart(2, '0')
  return `${date.getFullYear()}-${m}-${d}`
}

/** The Sunday that ends `today`'s week (today itself on a Sunday). */
export function endOfWeek(today: Date): string {
  const daysLeft = (7 - today.getDay()) % 7
  return localIso(new Date(today.getFullYear(), today.getMonth(), today.getDate() + daysLeft))
}

export function dueWhen(due: string, today: Date): DueWhen {
  const now = localIso(today)
  if (due < now) return 'overdue'
  if (due === now) return 'today'
  const tomorrow = localIso(new Date(today.getFullYear(), today.getMonth(), today.getDate() + 1))
  return due === tomorrow ? 'tomorrow' : 'later'
}

/** Parses YYYY-MM-DD as a local date, so the weekday doesn't shift with the time zone. */
export function parseDue(due: string): Date {
  const [y, m, d] = due.split('-').map(Number)
  return new Date(y, m - 1, d)
}
