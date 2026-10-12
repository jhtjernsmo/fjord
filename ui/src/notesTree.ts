// The notes list as a folder tree: folders are paths like "Work/Meetings", pinned notes on top.
import type { Note } from './api'

export type NoteSort = 'updated' | 'name' | 'created'

export interface FolderNode {
  name: string
  /** Full path, "" for the top level. */
  path: string
  notes: Note[]
  children: FolderNode[]
}

export type NoteRow =
  | { kind: 'folder'; path: string; name: string; depth: number; count: number; open: boolean }
  | { kind: 'note'; note: Note; depth: number }

const byName = (a: string, b: string) => a.localeCompare(b, undefined, { sensitivity: 'base', numeric: true })

export function sortNotes(notes: Note[], sort: NoteSort): Note[] {
  const list = [...notes]
  if (sort === 'name') return list.sort((a, b) => byName(a.title, b.title) || b.id - a.id)
  const key = sort === 'created' ? 'created_at' : 'updated_at'
  return list.sort((a, b) => b[key].localeCompare(a[key]) || b.id - a.id)
}

/** Pinned notes apart; everything else in a tree of folders. */
export function buildTree(notes: Note[], sort: NoteSort): { pinned: Note[]; root: FolderNode } {
  const root: FolderNode = { name: '', path: '', notes: [], children: [] }
  const folder = (path: string): FolderNode => {
    let node = root
    for (const part of path.split('/').filter(Boolean)) {
      const childPath = node.path ? `${node.path}/${part}` : part
      let child = node.children.find((c) => c.name === part)
      if (!child) {
        child = { name: part, path: childPath, notes: [], children: [] }
        node.children.push(child)
      }
      node = child
    }
    return node
  }
  const pinned: Note[] = []
  for (const note of sortNotes(notes, sort)) {
    if (note.pinned) pinned.push(note)
    else folder(note.folder).notes.push(note)
  }
  const sortChildren = (node: FolderNode) => {
    node.children.sort((a, b) => byName(a.name, b.name))
    node.children.forEach(sortChildren)
  }
  sortChildren(root)
  return { pinned, root }
}

const countNotes = (node: FolderNode): number => node.notes.length + node.children.reduce((n, c) => n + countNotes(c), 0)

/**
 * The rows to show, in order: top-level notes, then folders (subfolders before a folder's own
 * notes, like a file manager would list them). Notes inside a collapsed folder are left out.
 */
export function treeRows(root: FolderNode, collapsed: Set<string>): NoteRow[] {
  const rows: NoteRow[] = root.notes.map((note) => ({ kind: 'note', note, depth: 0 }))
  const walk = (node: FolderNode, depth: number) => {
    const open = !collapsed.has(node.path)
    rows.push({ kind: 'folder', path: node.path, name: node.name, depth, count: countNotes(node), open })
    if (!open) return
    node.children.forEach((c) => walk(c, depth + 1))
    rows.push(...node.notes.map((note) => ({ kind: 'note' as const, note, depth: depth + 1 })))
  }
  root.children.forEach((c) => walk(c, 0))
  return rows
}

/** Every folder path in use, with its parents, for suggestions. */
export function folderPaths(notes: Note[]): string[] {
  const all = new Set<string>()
  for (const n of notes) {
    const parts = n.folder.split('/').filter(Boolean)
    for (let i = 1; i <= parts.length; i++) all.add(parts.slice(0, i).join('/'))
  }
  return [...all].sort(byName)
}

/** The new path for a folder renamed from `from` to `to`, or the path unchanged if it's not under `from`. */
export function renamedPath(path: string, from: string, to: string): string {
  if (path === from) return to
  return path.startsWith(`${from}/`) ? `${to}${path.slice(from.length)}`.replace(/^\//, '') : path
}
