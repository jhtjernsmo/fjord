// How projects are ordered in the sidebar, the overview and the project picker.
// A per-device preference kept in localStorage, like the theme.
import { useCallback, useEffect, useMemo, useState } from 'react'
import type { ProjectSummary } from './api'

export type ProjectSortBy = 'numbered' | 'alphabetical' | 'recent' | 'created'
export const PROJECT_SORTS: ProjectSortBy[] = ['numbered', 'alphabetical', 'recent', 'created']

export interface ProjectSort {
  by: ProjectSortBy
  /** Reverse the natural order (Z–A, oldest first, …). */
  reversed: boolean
}

export const DEFAULT_PROJECT_SORT: ProjectSort = { by: 'numbered', reversed: false }

const SORT_KEY = 'fjord.projectSort'
const USED_KEY = 'fjord.projectsLastUsed'

/** Last-opened time per project id (ms). */
export type LastUsed = Record<number, number>

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' })
const byName = (a: ProjectSummary, b: ProjectSummary) => collator.compare(a.name.trim(), b.name.trim())

/** The number a name starts with ("12 Bokost" → 12), or null. */
export function leadingNumber(name: string): number | null {
  const match = /^\s*(\d+)/.exec(name)
  return match ? Number(match[1]) : null
}

/** Names that start with a number first, by that number; then the rest alphabetically. */
function byNumber(a: ProjectSummary, b: ProjectSummary): number {
  const na = leadingNumber(a.name)
  const nb = leadingNumber(b.name)
  if (na !== null && nb !== null && na !== nb) return na - nb
  if (na !== null && nb === null) return -1
  if (na === null && nb !== null) return 1
  return byName(a, b)
}

export function sortProjects(projects: ProjectSummary[], sort: ProjectSort, lastUsed: LastUsed = {}): ProjectSummary[] {
  const compare: Record<ProjectSortBy, (a: ProjectSummary, b: ProjectSummary) => number> = {
    numbered: byNumber,
    alphabetical: byName,
    // Most recently opened first; never-opened projects after, in numbered order.
    recent: (a, b) => (lastUsed[b.id] ?? 0) - (lastUsed[a.id] ?? 0) || byNumber(a, b),
    // Newest first.
    created: (a, b) => b.created_at.localeCompare(a.created_at) || byNumber(a, b),
  }
  const sorted = [...projects].sort(compare[sort.by])
  return sort.reversed ? sorted.reverse() : sorted
}

function read<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key)
    return raw ? { ...fallback, ...JSON.parse(raw) } : fallback
  } catch {
    return fallback
  }
}

function write(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    /* storage unavailable: the order just isn't remembered */
  }
}

/** Sorted projects plus the sort preference; records when `activeId` is opened. */
export function useProjectSort(projects: ProjectSummary[], activeId: number | null) {
  const [sort, setSortState] = useState<ProjectSort>(() => {
    const stored = read(SORT_KEY, DEFAULT_PROJECT_SORT)
    return PROJECT_SORTS.includes(stored.by) ? stored : DEFAULT_PROJECT_SORT
  })
  const [lastUsed, setLastUsed] = useState<LastUsed>(() => read(USED_KEY, {}))

  useEffect(() => {
    if (activeId === null) return
    setLastUsed((prev) => {
      const next = { ...prev, [activeId]: Date.now() }
      write(USED_KEY, next)
      return next
    })
  }, [activeId])

  const setSort = useCallback((next: ProjectSort) => {
    setSortState(next)
    write(SORT_KEY, next)
  }, [])

  // "Last used" re-sorts on open only when that's the chosen order, so the list doesn't jump otherwise.
  const usedKey = sort.by === 'recent' ? lastUsed : null
  const sorted = useMemo(() => sortProjects(projects, sort, usedKey ?? {}), [projects, sort, usedKey])
  return { sorted, sort, setSort }
}
