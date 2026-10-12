// The Git tab's branch list: a search box and a short list until you ask for all of them.
import type { GitBranch } from './api'

export const BRANCH_PREVIEW = 8

/** Branches whose name contains every word of the query (case-insensitive). */
export function matchBranches<T extends Pick<GitBranch, 'name'>>(branches: T[], query: string): T[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean)
  return words.length ? branches.filter((b) => words.every((w) => b.name.toLowerCase().includes(w))) : branches
}

/** What to show: every match while searching, otherwise the first few, keeping the current branch in view. */
export function visibleBranches<T extends Pick<GitBranch, 'name' | 'current'>>(branches: T[], query: string, showAll: boolean, limit = BRANCH_PREVIEW): T[] {
  const matches = matchBranches(branches, query)
  if (query.trim() || showAll || matches.length <= limit) return matches
  const first = matches.slice(0, limit)
  const current = matches.find((b) => b.current)
  return current && !first.includes(current) ? [...first.slice(0, limit - 1), current] : first
}
