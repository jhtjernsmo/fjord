import { describe, expect, test } from 'vitest'
import { prStatus, sortPullRequests } from '../src/prStatus'

type P = Parameters<typeof prStatus>[0] & { updated_at: string; number: number }
const pr = (number: number, extra: Partial<P> = {}): P => ({
  number,
  state: 'open',
  draft: false,
  checks: 'success',
  review: 'none',
  updated_at: `2026-10-${String(10 + number).padStart(2, '0')}T10:00:00Z`,
  ...extra,
})

describe('pull request status', () => {
  test('says what an open pull request needs', () => {
    expect(prStatus(pr(1, { review: 'approved' }))).toBe('ready')
    expect(prStatus(pr(1, { review: 'approved', checks: 'failure' }))).toBe('failing')
    expect(prStatus(pr(1, { review: 'changes_requested' }))).toBe('changes')
    expect(prStatus(pr(1, { review: 'required' }))).toBe('review')
    expect(prStatus(pr(1, { checks: 'pending' }))).toBe('running')
    expect(prStatus(pr(1))).toBeNull()
  })

  test('drafts and closed pull requests have no status', () => {
    expect(prStatus(pr(1, { draft: true, review: 'approved' }))).toBeNull()
    expect(prStatus(pr(1, { state: 'merged', review: 'approved' }))).toBeNull()
  })

  test('the ones that need something come first, closed last', () => {
    const list = [
      pr(1, { state: 'merged' }),
      pr(2, { draft: true }),
      pr(3),
      pr(4, { review: 'required' }),
      pr(5, { review: 'approved' }),
      pr(6, { checks: 'failure' }),
      pr(7, { review: 'changes_requested' }),
    ]
    expect(sortPullRequests(list).map((p) => p.number)).toEqual([5, 7, 6, 4, 3, 2, 1])
  })

  test('within a group, the most recently updated first', () => {
    expect(sortPullRequests([pr(1), pr(2)]).map((p) => p.number)).toEqual([2, 1])
  })
})
