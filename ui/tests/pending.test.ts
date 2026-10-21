import { describe, expect, test } from 'vitest'
import type { Status, Task } from '../src/api'
import { translator } from '../src/i18n'
import { pendingChip, pendingDays, waitedFor } from '../src/pending'
import { buildStandup, standupText } from '../src/standup'

const en = translator('en')
const no = translator('no')
const now = new Date('2026-10-21T12:00:00Z')

describe('pending days', () => {
  test('counts whole days since the task started waiting', () => {
    expect(pendingDays('2026-10-21T08:00:00Z', now)).toBe(0)
    expect(pendingDays('2026-10-20T11:00:00Z', now)).toBe(1)
    expect(pendingDays('2026-10-14T12:00:00Z', now)).toBe(7)
    expect(pendingDays('2026-10-22T12:00:00Z', now), 'clock skew never goes negative').toBe(0)
  })

  test('labels for cards and lists, in both languages', () => {
    expect(pendingChip('2026-10-21T08:00:00Z', en, now)).toBe('pending')
    expect(pendingChip('2026-10-18T12:00:00Z', en, now)).toBe('pending 3d')
    expect(pendingChip('2026-10-18T12:00:00Z', no, now)).toBe('venter 3 d')
    expect(waitedFor('2026-10-20T11:00:00Z', en, now)).toBe('1 day')
    expect(waitedFor('2026-10-18T12:00:00Z', no, now)).toBe('3 dager')
    expect(waitedFor('2026-10-21T08:00:00Z', no, now)).toBe('i dag')
  })
})

describe('pending tasks in the standup', () => {
  const status = (id: number, position: number, is_done = false) => ({ id, project_id: 1, name: `S${id}`, color: '#888', position, is_done }) as Status
  const task = (id: number, status_id: number, pending_since: string | null = null) =>
    ({ id, project_id: 1, status_id, title: `Task ${id}`, body_md: '', priority: 0, due_at: null, position: id, created_by: 'jonas', created_at: '', updated_at: '', archived_at: null, branch: null, parent_id: null, child_order: 0, pending_since }) as Task
  const statuses = [status(1, 0), status(2, 1), status(3, 2, true)]
  const tasks = [task(1, 2), task(2, 2, '2026-10-19T09:00:00Z'), task(3, 1, '2026-10-15T09:00:00Z'), task(4, 3, '2026-10-10T09:00:00Z')]
  const draft = buildStandup([], tasks, statuses, new Date(2026, 9, 19), 'jonas')

  test('are blockers, the longest waiting first, and not in progress', () => {
    expect(draft.pending.map((e) => e.task.id)).toEqual([3, 2])
    expect(draft.today.map((e) => e.task.id)).toEqual([1])
  })

  test('show up under blockers in the copied text', () => {
    const labels = { since: 'Since Monday', done: 'done', today: 'Today', blockers: 'Blockers', none: 'None', merged: 'merged', closed: 'closed', inReview: 'in review', mentioned: 'Mentioned in', subtasks: () => '', pendingFor: () => 'pending 2d' }
    const text = standupText({ ...draft, closedPrs: [], openPrs: [], mentions: [], githubMentions: [] }, labels)
    expect(text.split('Blockers\n')[1]).toBe('- Task 3 (#3), pending 2d\n- Task 2 (#2), pending 2d')
  })
})
