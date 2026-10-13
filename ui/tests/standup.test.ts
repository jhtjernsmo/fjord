// @vitest-environment jsdom
import { act, renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, test } from 'vitest'
import type { Activity, Mention, PullRequest, Status, Task } from '../src/api'
import type { StandupEntry } from '../src/standup'
import { buildStandup, isStandupDay, mentionsSince, myPullRequests, previousStandup, standupText, useStandupDays, useStandupProject } from '../src/standup'

const status = (id: number, position: number, is_done = false): Status =>
  ({ id, project_id: 1, name: `S${id}`, color: '#888', position, is_done }) as Status

const task = (id: number, status_id: number, extra: Partial<Task> = {}): Task =>
  ({ id, project_id: 1, status_id, title: `Task ${id}`, body_md: '', priority: 0, due_at: null, position: id, created_by: 'jonas', created_at: '', updated_at: '', archived_at: null, branch: null, parent_id: null, child_order: 0, ...extra }) as Task

const ids = (entries: StandupEntry[]) => entries.map((e) => e.task.id)

const act_ = (task_id: number, action: string, created_at: string, actor = 'jonas'): Activity =>
  ({ id: task_id * 100, project_id: 1, task_id, actor, action, subject: '', detail: null, created_at }) as Activity

// Monday 12 October 2026, standups on Monday, Wednesday and Friday.
const monday = new Date(2026, 9, 12, 9, 0)
const mwf = [1, 3, 5] as const

describe('standup days', () => {
  test('knows which days have a standup', () => {
    expect(isStandupDay(monday, [...mwf])).toBe(true)
    expect(isStandupDay(new Date(2026, 9, 13), [...mwf])).toBe(false)
  })

  test('the previous standup before Monday is Friday', () => {
    expect(previousStandup(monday, [...mwf])).toEqual(new Date(2026, 9, 9))
  })

  test('with one standup a week it goes a week back', () => {
    expect(previousStandup(monday, [1])).toEqual(new Date(2026, 9, 5))
  })
})

describe('buildStandup', () => {
  const statuses = [status(1, 0), status(2, 1), status(3, 2, true)]
  const tasks = [
    task(1, 3), // done since Friday
    task(2, 2), // in progress, touched
    task(3, 1, { due_at: '2026-10-12' }), // backlog, due today
    task(4, 2), // in progress, not touched
    task(5, 1), // only touched by the agent
    task(6, 3), // done before Friday
  ]
  const activity = [
    act_(1, 'task.move', '2026-10-09T13:00:00'),
    act_(2, 'task.branch', '2026-10-10T10:00:00'),
    act_(5, 'task.create', '2026-10-10T10:00:00', 'claude'),
    act_(6, 'task.move', '2026-10-08T10:00:00'),
    act_(2, 'file.attach', '2026-10-10T11:00:00'),
  ]
  const draft = buildStandup(activity, tasks, statuses, new Date(2026, 9, 9), 'jonas')

  test('splits your own work since the last standup into done and worked on', () => {
    expect(ids(draft.done)).toEqual([1])
    expect(ids(draft.workedOn)).toEqual([2])
  })

  test("today lists what's in progress, not the backlog", () => {
    expect(ids(draft.today)).toEqual([2, 4])
  })

  test('formats as plain text with pull requests, mentions and an empty blockers line', () => {
    const full = { ...draft, closedPrs: [pr(12, 'merged', 'jhtjernsmo', '2026-10-10T10:00:00Z')], openPrs: [pr(14, 'open', 'jhtjernsmo', '2026-10-11T10:00:00Z')], mentions: [mention(4211, '2026-10-10T08:00:00Z')], githubMentions: [{ repo: 'acme/api', number: 7, title: 'Flaky login test', url: 'https://github.com/acme/api/issues/7', by: 'ola', at: '2026-10-10T09:00:00Z', pull_request: false }] }
    const labels = { since: 'Since Friday', done: 'done', today: 'Today', blockers: 'Blockers', none: 'None', merged: 'merged', closed: 'closed', inReview: 'in review', mentioned: 'Mentioned in', subtasks: (done: number, total: number) => `${done}/${total} subtasks done` }
    expect(standupText(full, labels)).toBe(
      [
        'Since Friday',
        '- Task 1 (#1) – done',
        '- Task 2 (#2)',
        '- PR #12 PR 12 – merged',
        '- Mentioned in #4211 Item 4211 (Kari)',
        '- Mentioned in acme/api#7 Flaky login test (ola)',
        '',
        'Today',
        '- Task 2 (#2)',
        '- Task 4 (#4)',
        '- PR #14 PR 14 – in review',
        '',
        'Blockers',
        '- None',
      ].join('\n'),
    )
  })
})

describe('buildStandup with subtasks', () => {
  const statuses = [status(1, 0), status(2, 1), status(3, 2, true)]
  const tasks = [
    task(10, 2), // parent in progress, not touched itself
    task(11, 3, { parent_id: 10 }), // subtask finished since Friday
    task(12, 2, { parent_id: 10 }), // subtask worked on
    task(13, 1, { parent_id: 10 }), // subtask not touched
    task(20, 3), // parent done, touched
    task(21, 3, { parent_id: 20 }),
    task(30, 2, { parent_id: 99 }), // parent archived or gone
    task(99, 2, { archived_at: '2026-10-01' }),
  ]
  const activity = [
    act_(11, 'task.move', '2026-10-09T13:00:00'),
    act_(12, 'task.update', '2026-10-10T10:00:00'),
    act_(20, 'task.move', '2026-10-10T11:00:00'),
    act_(21, 'task.move', '2026-10-10T11:00:00'),
    act_(30, 'task.update', '2026-10-10T12:00:00'),
  ]
  const draft = buildStandup(activity, tasks, statuses, new Date(2026, 9, 9), 'jonas')

  test('subtasks never get lines of their own, their parent does', () => {
    expect(ids(draft.done)).toEqual([20])
    expect(ids(draft.workedOn)).toEqual([10, 30])
  })

  test('the parent carries the subtasks you touched and how far along all of them are', () => {
    const parent = draft.workedOn[0]
    expect(parent.subtasks.map((s) => [s.task.id, s.done])).toEqual([
      [11, true],
      [12, false],
    ])
    expect(parent.progress).toEqual({ done: 1, total: 3 })
    expect(draft.done[0].subtasks.map((s) => s.task.id)).toEqual([21])
  })

  test('a subtask whose parent is archived stands on its own', () => {
    expect(draft.workedOn[1]).toEqual({ task: tasks[6], subtasks: [], progress: null })
  })

  test('today shows parents with progress, not subtasks', () => {
    expect(ids(draft.today)).toEqual([10, 30])
    expect(draft.today[0].progress).toEqual({ done: 1, total: 3 })
  })

  test('the text has one line per parent with its progress', () => {
    const labels = { since: 'Since Friday', done: 'done', today: 'Today', blockers: 'Blockers', none: 'None', merged: 'merged', closed: 'closed', inReview: 'in review', mentioned: 'Mentioned in', subtasks: (done: number, total: number) => `${done}/${total} subtasks done` }
    const text = standupText({ ...draft, closedPrs: [], openPrs: [], mentions: [], githubMentions: [] }, labels)
    expect(text.split('\n').slice(0, 4)).toEqual(['Since Friday', '- Task 20 (#20), 1/1 subtasks done – done', '- Task 10 (#10), 1/3 subtasks done', '- Task 30 (#30)'])
  })
})

const pr = (number: number, state: PullRequest['state'], author: string, updated_at: string): PullRequest =>
  ({ number, title: `PR ${number}`, state, draft: false, url: `https://github.com/o/r/pull/${number}`, author, head: 'h', base: 'main', head_sha: 'x', updated_at, checks: 'none' }) as PullRequest

const mention = (id: number, at: string, project = 'Platform'): Mention =>
  ({ org: 'contoso', id, project, kind: 'Bug', state: 'Active', title: `Item ${id}`, url: `https://dev.azure.com/contoso/${project}/_workitems/edit/${id}`, by: 'Kari', at, snippet: '' }) as Mention

describe('myPullRequests', () => {
  const friday = new Date('2026-10-09T00:00:00Z')
  const prs = [
    pr(1, 'open', 'jhtjernsmo', '2026-10-01T00:00:00Z'),
    pr(2, 'merged', 'JHTjernsmo', '2026-10-10T00:00:00Z'),
    pr(3, 'merged', 'jhtjernsmo', '2026-10-05T00:00:00Z'),
    pr(4, 'open', 'someone-else', '2026-10-11T00:00:00Z'),
    pr(5, 'closed', 'Jonas Tjernsmo', '2026-10-10T00:00:00Z'),
  ]

  test('keeps only your own, matching GitHub login or Azure display name', () => {
    const mine = myPullRequests(prs, ['jhtjernsmo', 'Jonas Tjernsmo'], friday)
    expect(mine.openPrs.map((p) => p.number)).toEqual([1])
    expect(mine.closedPrs.map((p) => p.number)).toEqual([2, 5])
  })

  test('without a known account there is nothing', () => {
    expect(myPullRequests(prs, [], friday)).toEqual({ openPrs: [], closedPrs: [] })
  })
})

describe('mentionsSince', () => {
  const friday = new Date('2026-10-09T00:00:00Z')
  const list = [mention(1, '2026-10-08T00:00:00Z'), mention(2, '2026-10-10T00:00:00Z'), mention(3, '2026-10-11T00:00:00Z', 'Mobile')]

  test('takes mentions since the last standup, newest first', () => {
    expect(mentionsSince(list, friday, null).map((m) => m.id)).toEqual([3, 2])
  })

  test('limits them to the Azure projects mapped to the chosen Fjord project', () => {
    expect(mentionsSince(list, friday, [{ org: 'Contoso', project: 'Platform' }]).map((m) => m.id)).toEqual([2])
  })
})

describe('useStandupDays', () => {
  beforeEach(() => localStorage.clear())

  test('starts empty and keeps two hooks in sync', () => {
    const settings = renderHook(() => useStandupDays())
    const overview = renderHook(() => useStandupDays())
    expect(overview.result.current[0]).toEqual([])

    act(() => settings.result.current[1](1))
    act(() => settings.result.current[1](3))

    expect(overview.result.current[0]).toEqual([1, 3])
    expect(JSON.parse(localStorage.getItem('fjord.standupDays') ?? '')).toEqual([1, 3])
  })

  test('the standup project defaults to all and can be set', () => {
    const { result } = renderHook(() => useStandupProject())
    expect(result.current[0]).toBeNull()
    act(() => result.current[1](2))
    expect(result.current[0]).toBe(2)
  })

  test('ignores junk in storage', () => {
    localStorage.setItem('fjord.standupDays', '[1, "x", 9, 5]')
    const { result } = renderHook(() => useStandupDays())
    expect(result.current[0]).toEqual([1, 5])
  })
})
