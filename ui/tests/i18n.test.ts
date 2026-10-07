import { describe, expect, it } from 'vitest'
import { describeActivity, formatBytes, isOverdue } from '../src/api'
import type { Activity, Status, Task } from '../src/api'
import { translator } from '../src/i18n'
import type { MessageKey } from '../src/i18n'
import { DEFAULT_ACTIONS } from '../src/keymap'

const en = translator('en')
const no = translator('no')

describe('translator', () => {
  it('interpolates variables and falls back to English', () => {
    expect(en('home.open', { open: 3, projects: 2 })).toBe('You have 3 open tasks across 2 projects.')
    expect(no('home.open', { open: 3, projects: 2 })).toContain('3 åpne oppgaver')
    expect(en('tab.board')).toBe('Board')
    expect(no('tab.board')).toBe('Tavle')
  })

  it('has an English label and group for every keyboard action', () => {
    for (const a of DEFAULT_ACTIONS) {
      expect(en(`action.${a.id}` as MessageKey), a.id).not.toBe(`action.${a.id}`)
      expect(en(`group.${a.group}` as MessageKey)).not.toBe(`group.${a.group}`)
    }
  })

  it('translates every action into Norwegian too', () => {
    for (const a of DEFAULT_ACTIONS) {
      expect(no(`action.${a.id}` as MessageKey), a.id).not.toBe(en(`action.${a.id}` as MessageKey))
    }
  })
})

describe('describeActivity', () => {
  const base: Activity = { id: 1, project_id: 1, task_id: 1, actor: 'claude', action: 'task.move', subject: 'Ship', detail: 'Done', created_at: '' }
  it('renders language-neutral activity in the chosen language', () => {
    expect(describeActivity(base, en)).toBe('moved “Ship” to Done')
    expect(describeActivity(base, no)).toBe('flyttet «Ship» til Done')
    expect(describeActivity({ ...base, action: 'status.create', subject: 'Review' }, en)).toBe('added column “Review”')
  })
})

describe('helpers', () => {
  it('formats byte sizes', () => {
    expect(formatBytes(512)).toBe('512 B')
    expect(formatBytes(2048)).toBe('2.0 KB')
    expect(formatBytes(5 * 1024 * 1024)).toBe('5.0 MB')
  })

  it('flags overdue tasks only when not done', () => {
    const statuses: Status[] = [
      { id: 1, project_id: 1, name: 'To do', color: '', position: 0, is_done: false },
      { id: 2, project_id: 1, name: 'Done', color: '', position: 1, is_done: true },
    ]
    const task = { status_id: 1, due_at: '2000-01-01' } as Task
    expect(isOverdue(task, statuses)).toBe(true)
    expect(isOverdue({ ...task, status_id: 2 }, statuses)).toBe(false)
    expect(isOverdue({ ...task, due_at: '2999-01-01' }, statuses)).toBe(false)
    expect(isOverdue({ ...task, due_at: null }, statuses)).toBe(false)
  })
})
