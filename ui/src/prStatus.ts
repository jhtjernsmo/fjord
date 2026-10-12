// What a pull request needs next, and which ones to show first in the Git tab.
import type { PullRequest } from './api'

export type PrStatus = 'ready' | 'changes' | 'failing' | 'review' | 'running' | null

/** The one thing worth saying about an open pull request, or null. */
export function prStatus(pr: Pick<PullRequest, 'state' | 'draft' | 'checks' | 'review'>): PrStatus {
  if (pr.state !== 'open' || pr.draft) return null
  if (pr.checks === 'failure') return 'failing'
  if (pr.review === 'changes_requested') return 'changes'
  if (pr.review === 'required') return 'review'
  if (pr.checks === 'pending') return 'running'
  if (pr.review === 'approved') return 'ready'
  return null
}

const RANK: Record<string, number> = { ready: 0, changes: 1, failing: 2, review: 3, running: 4, none: 5 }

/** Open pull requests that need something first, then other open ones, drafts, and closed or merged last. */
export function sortPullRequests<T extends Pick<PullRequest, 'state' | 'draft' | 'checks' | 'review' | 'updated_at'>>(prs: T[]): T[] {
  const rank = (pr: T) => {
    if (pr.state !== 'open') return 7
    if (pr.draft) return 6
    return RANK[prStatus(pr) ?? 'none']
  }
  return [...prs].sort((a, b) => rank(a) - rank(b) || b.updated_at.localeCompare(a.updated_at))
}

export const PR_PREVIEW = 5
