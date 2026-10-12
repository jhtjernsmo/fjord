import { describe, expect, test } from 'vitest'
import type { Note } from '../src/api'
import { buildTree, folderPaths, renamedPath, sortNotes, treeRows } from '../src/notesTree'

const note = (id: number, title: string, folder = '', extra: Partial<Note> = {}): Note => ({
  id,
  project_id: null,
  title,
  body_md: '',
  folder,
  pinned: false,
  locked: false,
  created_at: `2026-10-0${id}T10:00:00Z`,
  updated_at: `2026-10-1${9 - id}T10:00:00Z`,
  ...extra,
})

const notes = [
  note(1, 'Retro', 'Work'),
  note(2, 'Standup 12.10', 'Work/Meetings'),
  note(3, 'Ideas'),
  note(4, 'Groceries', 'Home'),
  note(5, 'Launch plan', 'Work', { pinned: true }),
]

describe('notes tree', () => {
  test('sorts by name, created or updated', () => {
    expect(sortNotes(notes, 'name').map((n) => n.title)).toEqual(['Groceries', 'Ideas', 'Launch plan', 'Retro', 'Standup 12.10'])
    expect(sortNotes(notes, 'created').map((n) => n.id)).toEqual([5, 4, 3, 2, 1])
    expect(sortNotes(notes, 'updated').map((n) => n.id)).toEqual([1, 2, 3, 4, 5])
  })

  test('pinned notes apart, folders nested and alphabetical', () => {
    const { pinned, root } = buildTree(notes, 'name')
    expect(pinned.map((n) => n.title)).toEqual(['Launch plan'])
    expect(root.notes.map((n) => n.title)).toEqual(['Ideas'])
    expect(root.children.map((c) => c.path)).toEqual(['Home', 'Work'])
    expect(root.children[1].children[0].path).toBe('Work/Meetings')
  })

  test('rows list top-level notes first, then folders, subfolders before their notes', () => {
    const { root } = buildTree(notes, 'name')
    const rows = treeRows(root, new Set())
    expect(rows.map((r) => (r.kind === 'folder' ? `[${r.path}] ${r.count}` : `${'  '.repeat(r.depth)}${r.note.title}`))).toEqual([
      'Ideas',
      '[Home] 1',
      '  Groceries',
      '[Work] 2',
      '[Work/Meetings] 1',
      '    Standup 12.10',
      '  Retro',
    ])
  })

  test('a collapsed folder hides everything under it', () => {
    const { root } = buildTree(notes, 'name')
    const rows = treeRows(root, new Set(['Work']))
    expect(rows.filter((r) => r.kind === 'note').map((r) => (r.kind === 'note' ? r.note.title : ''))).toEqual(['Ideas', 'Groceries'])
    expect(rows.find((r) => r.kind === 'folder' && r.path === 'Work')).toMatchObject({ open: false, count: 2 })
  })

  test('folder suggestions and renamed paths', () => {
    expect(folderPaths(notes)).toEqual(['Home', 'Work', 'Work/Meetings'])
    expect(renamedPath('Work/Meetings', 'Work', 'Jobb')).toBe('Jobb/Meetings')
    expect(renamedPath('Workshop', 'Work', 'Jobb')).toBe('Workshop')
    expect(renamedPath('Work/Meetings', 'Work', '')).toBe('Meetings')
  })
})
