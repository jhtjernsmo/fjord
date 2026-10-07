// Notes list + editor. Used for the global notespace and for a project's Notes tab.
import { useEffect, useMemo, useState } from 'react'
import { NotebookText, Pin, Plus, Search } from 'lucide-react'
import { api, relativeTime } from '../api'
import type { Note, ProjectSummary } from '../api'
import { useApp, useLive } from '../data'
import { useContextMenus } from './actions'
import { Backlinks, NoteEditor } from './NoteEditor'

interface Props {
  /** null = global notespace (all notes, free ones first); a number = that project's notes. */
  projectId: number | null
  projects: ProjectSummary[]
  selectedId?: number | null
  onSelect?: (id: number | null) => void
}

export function Notespace({ projectId, projects, selectedId, onSelect }: Props) {
  const { run, t, locale } = useApp()
  const { noteMenu } = useContextMenus()
  const [notes] = useLive(() => (projectId === null ? api.listAllNotes(false) : api.listNotes(projectId)), [projectId])
  const [localId, setLocalId] = useState<number | null>(null)
  const [filter, setFilter] = useState('')
  const activeId = selectedId ?? localId
  const select = (id: number | null) => (onSelect ? onSelect(id) : setLocalId(id))
  const active: Note | undefined = notes?.find((n) => n.id === activeId) ?? (activeId == null ? notes?.[0] : undefined)

  useEffect(() => {
    if (activeId != null && notes && !notes.some((n) => n.id === activeId)) select(null)
  }, [notes]) // eslint-disable-line react-hooks/exhaustive-deps

  const groups = useMemo(() => {
    const needle = filter.trim().toLowerCase()
    const shown = (notes ?? []).filter((n) => !needle || n.title.toLowerCase().includes(needle) || n.body_md.toLowerCase().includes(needle))
    const byFolder = new Map<string, Note[]>()
    for (const n of shown) {
      const key = n.pinned ? '\u0000pinned' : n.folder
      byFolder.set(key, [...(byFolder.get(key) ?? []), n])
    }
    // Pinned first, then notes without a folder, then folders alphabetically.
    const rank = (k: string) => (k === '\u0000pinned' ? 0 : k === '' ? 1 : 2)
    return [...byFolder.entries()].sort(([a], [b]) => rank(a) - rank(b) || a.localeCompare(b))
  }, [notes, filter])

  const projectName = (id: number | null) => projects.find((p) => p.id === id)?.name

  const create = async () => {
    const note = await run(api.createNote(projectId, t('notes.untitled'), ''))
    if (note) select(note.id)
  }

  return (
    <div className="notespace">
      <aside className="note-list">
        <button className="btn primary" onClick={create}>
          <Plus size={14} /> {t('notes.new').replace('+ ', '')}
        </button>
        <label className="filter-search small-search">
          <Search size={13} />
          <input value={filter} onChange={(e) => setFilter(e.target.value)} placeholder={t('notes.filter')} aria-label={t('notes.filter')} />
        </label>
        {groups.map(([folder, items]) => (
          <div key={folder} className="note-group">
            {(folder || groups.length > 1) && (
              <div className="note-folder">
                {folder === '\u0000pinned' ? (
                  <>
                    <Pin size={11} /> {t('notes.pinned')}
                  </>
                ) : (
                  folder || t('notes.unsorted')
                )}
              </div>
            )}
            {items.map((n) => (
              <button
                key={n.id}
                className={`nav-item ${n.id === active?.id ? 'active' : ''}`}
                onClick={() => select(n.id)}
                onContextMenu={noteMenu(n, () => select(n.id))}
              >
                <NotebookText size={15} strokeWidth={1.6} />
                <span className="name">{n.title}</span>
                {projectId === null && n.project_id !== null && <span className="note-project">{projectName(n.project_id)}</span>}
                <span className="note-time">{relativeTime(n.updated_at, t, locale)}</span>
              </button>
            ))}
          </div>
        ))}
        {notes && notes.length === 0 && <div className="hint">{t('notes.none')}</div>}
        {projectId !== null && <Backlinks kind="project" id={projectId} />}
      </aside>
      {active ? (
        <NoteEditor key={active.id} note={active} projects={projects} onDeleted={() => select(null)} />
      ) : (
        <div className="empty-state">
          <NotebookText size={36} strokeWidth={1.2} />
          <div>{t('notes.emptyHint')}</div>
        </div>
      )}
    </div>
  )
}
