// Notes list + editor. Used for the global notespace and for a project's Notes tab.
// The list is a tree of folders (paths like "Work/Meetings"); drag a note onto a folder to move it.
import { useEffect, useMemo, useRef, useState } from 'react'
import type { ReactNode } from 'react'
import { DndContext, PointerSensor, useDraggable, useDroppable, useSensor, useSensors } from '@dnd-kit/core'
import type { DragEndEvent } from '@dnd-kit/core'
import { ChevronDown, ChevronRight, Folder, Lock, NotebookText, Pin, Plus, Search } from 'lucide-react'
import { api, relativeTime } from '../api'
import type { Note, ProjectSummary } from '../api'
import { useActions, useApp, useLive } from '../data'
import type { MessageKey } from '../i18n'
import { buildTree, treeRows } from '../notesTree'
import type { NoteSort } from '../notesTree'
import { useStoredPref } from '../storedPref'
import { useContextMenus } from './actions'
import { Backlinks, NoteEditor } from './NoteEditor'

interface Props {
  /** null = global notespace (all notes, free ones first); a number = that project's notes. */
  projectId: number | null
  projects: ProjectSummary[]
  selectedId?: number | null
  onSelect?: (id: number | null) => void
}

const DRAG_ACTIVATION_PX = 6
const SORTS: NoteSort[] = ['updated', 'name', 'created']
const TOP_LEVEL = '\u0000top'

const parseSort = (raw: unknown): NoteSort => (SORTS.includes(raw as NoteSort) ? (raw as NoteSort) : 'updated')
const parseCollapsed = (raw: unknown): string[] => (Array.isArray(raw) ? raw.filter((p): p is string => typeof p === 'string') : [])
const NONE: string[] = []

function DraggableNote({ note, children }: { note: Note; children: ReactNode }) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({ id: `note-${note.id}`, data: { note } })
  return (
    <div ref={setNodeRef} {...attributes} {...listeners} className={isDragging ? 'note-dragging' : ''}>
      {children}
    </div>
  )
}

function FolderDrop({ path, children }: { path: string; children: ReactNode }) {
  const { setNodeRef, isOver } = useDroppable({ id: `folder-${path}`, data: { path } })
  return (
    <div ref={setNodeRef} className={isOver ? 'drop-over' : ''}>
      {children}
    </div>
  )
}

export function Notespace({ projectId, projects, selectedId, onSelect }: Props) {
  const { run, t, locale } = useApp()
  const { noteMenu } = useContextMenus()
  const [notes] = useLive(() => (projectId === null ? api.listAllNotes(false) : api.listNotes(projectId)), [projectId])
  const [localId, setLocalId] = useState<number | null>(null)
  const [filter, setFilter] = useState('')
  const [sort, setSort] = useStoredPref('fjord.noteSort', parseSort, 'updated')
  const [collapsedList, setCollapsed] = useStoredPref('fjord.noteFoldersCollapsed', parseCollapsed, NONE)
  const [renaming, setRenaming] = useState<{ path: string; name: string } | null>(null)
  const [dragging, setDragging] = useState(false)
  const filterRef = useRef<HTMLInputElement>(null)
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: DRAG_ACTIVATION_PX } }))
  const activeId = selectedId ?? localId
  const select = (id: number | null) => (onSelect ? onSelect(id) : setLocalId(id))
  const active: Note | undefined = notes?.find((n) => n.id === activeId) ?? (activeId == null ? notes?.[0] : undefined)

  useEffect(() => {
    if (activeId != null && notes && !notes.some((n) => n.id === activeId)) select(null)
  }, [notes]) // eslint-disable-line react-hooks/exhaustive-deps

  const needle = filter.trim().toLowerCase()
  const { pinned, rows } = useMemo(() => {
    const shown = (notes ?? []).filter((n) => !needle || n.title.toLowerCase().includes(needle) || n.body_md.toLowerCase().includes(needle))
    const tree = buildTree(shown, sort)
    // While filtering, every folder is open so matches aren't hidden.
    return { pinned: tree.pinned, rows: treeRows(tree.root, new Set(needle ? [] : collapsedList)) }
  }, [notes, needle, sort, collapsedList])

  const projectName = (id: number | null) => projects.find((p) => p.id === id)?.name

  // j/k walk the notes in the order they're shown.
  const ordered = [...pinned, ...rows.flatMap((r) => (r.kind === 'note' ? [r.note] : []))]
  const step = (delta: number) => {
    if (ordered.length === 0) return
    const i = ordered.findIndex((n) => n.id === active?.id)
    select(ordered[Math.min(Math.max(i + delta, 0), ordered.length - 1)].id)
  }

  const create = async () => {
    const note = await run(api.createNote(projectId, t('notes.untitled'), ''))
    if (note) select(note.id)
  }

  const toggleFolder = (path: string) => setCollapsed((list) => (list.includes(path) ? list.filter((p) => p !== path) : [...list, path]))

  const renameFolder = async (path: string, name: string) => {
    setRenaming(null)
    const clean = name.trim().replace(/\//g, '-')
    const parent = path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : ''
    const to = clean ? (parent ? `${parent}/${clean}` : clean) : parent
    if (!clean || to === path) return
    // A folder can hold notes from several projects in the global notespace; rename it in each.
    const scopes = projectId !== null ? [projectId] : [...new Set((notes ?? []).filter((n) => n.folder === path || n.folder.startsWith(`${path}/`)).map((n) => n.project_id))]
    for (const scope of scopes) await run(api.renameNoteFolder(scope, path, to))
  }

  const onDragEnd = (e: DragEndEvent) => {
    setDragging(false)
    const note = e.active.data.current?.note as Note | undefined
    const target = e.over?.data.current?.path as string | undefined
    if (!note || target === undefined) return
    const folder = target === TOP_LEVEL ? '' : target
    if (folder !== note.folder) run(api.setNoteFolder(note.id, folder))
  }

  useActions(
    {
      'notes.new': () => void create(),
      'notes.next': () => step(1),
      'notes.prev': () => step(-1),
      'notes.filter': () => filterRef.current?.focus(),
      'notes.pin': () => active && run(api.setNotePinned(active.id, !active.pinned)),
      'notes.lock': () => active && run(api.setNoteLocked(active.id, !active.locked)),
    },
    'notes',
  )

  const noteRow = (n: Note, depth: number) => (
    <DraggableNote key={n.id} note={n}>
      <button
        className={`nav-item note-row ${n.id === active?.id ? 'active' : ''}`}
        style={{ paddingLeft: 10 + depth * 14 }}
        onClick={() => select(n.id)}
        onContextMenu={noteMenu(n, () => select(n.id))}
      >
        {n.locked ? <Lock size={14} strokeWidth={1.6} /> : <NotebookText size={15} strokeWidth={1.6} />}
        <span className="name">{n.title}</span>
        {projectId === null && n.project_id !== null && <span className="note-project">{projectName(n.project_id)}</span>}
        <span className="note-time">{relativeTime(n.updated_at, t, locale)}</span>
      </button>
    </DraggableNote>
  )

  return (
    <div className="notespace">
      <aside className="note-list">
        <button className="btn primary" onClick={create}>
          <Plus size={14} /> {t('notes.new').replace('+ ', '')}
        </button>
        <div className="note-tools">
          <label className="filter-search small-search">
            <Search size={13} />
            <input ref={filterRef} value={filter} onChange={(e) => setFilter(e.target.value)} placeholder={t('notes.filter')} aria-label={t('notes.filter')} />
          </label>
          <select className="input note-sort" value={sort} onChange={(e) => setSort(() => e.target.value as NoteSort)} aria-label={t('notes.sort')} title={t('notes.sort')}>
            {SORTS.map((s) => (
              <option key={s} value={s}>
                {t(`notes.sort.${s}` as MessageKey)}
              </option>
            ))}
          </select>
        </div>
        <DndContext sensors={sensors} onDragStart={() => setDragging(true)} onDragCancel={() => setDragging(false)} onDragEnd={onDragEnd}>
          {pinned.length > 0 && (
            <div className="note-group">
              <div className="note-folder">
                <Pin size={11} /> {t('notes.pinned')}
              </div>
              {pinned.map((n) => noteRow(n, 0))}
            </div>
          )}
          {rows.map((r) =>
            r.kind === 'note' ? (
              noteRow(r.note, r.depth)
            ) : (
              <FolderDrop key={`f-${r.path}`} path={r.path}>
                <div className="note-folder folder-row" style={{ paddingLeft: 6 + r.depth * 14 }}>
                  <button className="folder-toggle" onClick={() => toggleFolder(r.path)} aria-expanded={r.open} aria-label={r.name}>
                    {r.open ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                    <Folder size={12} />
                  </button>
                  {renaming?.path === r.path ? (
                    <input
                      className="input folder-rename"
                      autoFocus
                      value={renaming.name}
                      onChange={(e) => setRenaming({ path: r.path, name: e.target.value })}
                      onBlur={() => renameFolder(r.path, renaming.name)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') e.currentTarget.blur()
                        if (e.key === 'Escape') setRenaming(null)
                        e.stopPropagation()
                      }}
                      aria-label={t('notes.renameFolder')}
                    />
                  ) : (
                    <span className="folder-name" onClick={() => toggleFolder(r.path)} onDoubleClick={() => setRenaming({ path: r.path, name: r.name })} title={t('notes.renameFolderHint')}>
                      {r.name}
                    </span>
                  )}
                  <span className="folder-count">{r.count}</span>
                </div>
              </FolderDrop>
            ),
          )}
          {dragging && (
            <FolderDrop path={TOP_LEVEL}>
              <div className="note-folder note-drop-top">{t('notes.noFolder')}</div>
            </FolderDrop>
          )}
        </DndContext>
        {notes && notes.length === 0 && <div className="hint">{t('notes.none')}</div>}
        {projectId !== null && <Backlinks kind="project" id={projectId} />}
      </aside>
      {active ? (
        <NoteEditor key={active.id} note={active} projects={projects} folders={notes ?? []} onDeleted={() => select(null)} />
      ) : (
        <div className="empty-state">
          <NotebookText size={36} strokeWidth={1.2} />
          <div>{t('notes.emptyHint')}</div>
        </div>
      )}
    </div>
  )
}
