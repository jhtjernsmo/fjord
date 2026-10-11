// @vitest-environment jsdom
import { act, renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, test } from 'vitest'
import type { Activity, Status, Task } from '../src/api'
import { buildStandup, isStandupDay, previousStandup, standupText, useStandupDays } from '../src/standup'

const status = (id: number, position: number, is_done = false): Status =>
  ({ id, project_id: 1, name: `S${id}`, color: '#888', position, is_done }) as Status

const task = (id: number, status_id: number, extra: Partial<Task> = {}): Task =>
  ({ id, project_id: 1, status_id, title: `Task ${id}`, body_md: '', priority: 0, due_at: null, position: id, created_by: 'jonas', created_at: '', updated_at: '', archived_at: null, branch: null, parent_id: null, child_order: 0, ...extra }) as Task

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
    expect(draft.done.map((t) => t.id)).toEqual([1])
    expect(draft.workedOn.map((t) => t.id)).toEqual([2])
  })

  test("today lists what's in progress, not the backlog", () => {
    expect(draft.today.map((t) => t.id)).toEqual([2, 4])
  })

  test('formats as plain text with an empty blockers line', () => {
    const text = standupText(draft, { since: 'Since Friday', done: 'done', today: 'Today', blockers: 'Blockers', none: 'None' })
    expect(text).toBe(
      ['Since Friday', '- Task 1 (#1) – done', '- Task 2 (#2)', '', 'Today', '- Task 2 (#2)', '- Task 4 (#4)', '', 'Blockers', '- None'].join('\n'),
    )
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

  test('ignores junk in storage', () => {
    localStorage.setItem('fjord.standupDays', '[1, "x", 9, 5]')
    const { result } = renderHook(() => useStandupDays())
    expect(result.current[0]).toEqual([1, 5])
  })
})
