// Secondary project tabs: notes, files and activity.
import { useEffect, useState } from 'react'
import Markdown from 'react-markdown'
import { api, relativeTime } from '../api'
import type { Note } from '../api'
import { useApp, useLive } from '../data'
import { ActivityList } from './Home'
import { FileTile } from './TaskPanel'

export function NotesView({ projectId }: { projectId: number }) {
  const { run } = useApp()
  const [notes] = useLive(() => api.listNotes(projectId), [projectId])
  const [activeId, setActiveId] = useState<number | null>(null)
  const active: Note | undefined = notes?.find((n) => n.id === activeId) ?? notes?.[0]
  const [title, setTitle] = useState('')
  const [body, setBody] = useState('')
  const [editing, setEditing] = useState(false)

  useEffect(() => {
    setTitle(active?.title ?? '')
    setBody(active?.body_md ?? '')
  }, [active?.id, active?.title, active?.body_md])

  const create = async () => {
    const note = await run(api.addNote(projectId, 'Nytt notat', ''))
    if (note) {
      setActiveId(note.id)
      setEditing(true)
    }
  }
  const save = () => active && title.trim() && run(api.updateNote(active.id, title, body))

  return (
    <div className="page">
      <div className="notes">
        <div className="note-list">
          <button className="btn primary" onClick={create}>
            + Nytt notat
          </button>
          {(notes ?? []).map((n) => (
            <button key={n.id} className={`nav-item ${n.id === active?.id ? 'active' : ''}`} onClick={() => setActiveId(n.id)}>
              <span>📝</span>
              <span className="name">{n.title}</span>
              <span className="sub" style={{ fontSize: 11, color: 'var(--dim)' }}>
                {relativeTime(n.updated_at)}
              </span>
            </button>
          ))}
        </div>
        {active ? (
          <div className="note-editor">
            <input className="title-input" value={title} onChange={(e) => setTitle(e.target.value)} onBlur={save} aria-label="Tittel" />
            <div className="section-title">
              Innhold
              <button onClick={() => (editing ? (save(), setEditing(false)) : setEditing(true))}>{editing ? 'Lagre' : 'Rediger'}</button>
            </div>
            {editing ? (
              <textarea
                className="md-editor"
                style={{ minHeight: 360 }}
                autoFocus
                value={body}
                onChange={(e) => setBody(e.target.value)}
                onBlur={() => {
                  save()
                  setEditing(false)
                }}
              />
            ) : (
              <div className={`markdown ${active.body_md ? '' : 'empty'}`} onClick={() => setEditing(true)}>
                {active.body_md ? <Markdown>{active.body_md}</Markdown> : 'Tomt notat. Klikk for å skrive.'}
              </div>
            )}
          </div>
        ) : (
          <div className="empty-state">Ingen notater ennå.</div>
        )}
      </div>
    </div>
  )
}

export function FilesView({ projectId }: { projectId: number }) {
  const { run } = useApp()
  const [files] = useLive(() => api.listAttachments(projectId), [projectId])
  return (
    <div className="page">
      <div className="dropzone" style={{ marginBottom: 16 }}>
        Dra filer inn i vinduet for å legge dem ved prosjektet
      </div>
      {files && files.length > 0 ? (
        <div className="files">
          {files.map((f) => (
            <FileTile key={f.id} file={f} onDetach={() => run(api.detachFile(f.id))} />
          ))}
        </div>
      ) : (
        <div className="empty-state">
          <span className="big">📎</span>Ingen filer ennå.
        </div>
      )}
    </div>
  )
}

export function ActivityView({ projectId }: { projectId: number }) {
  const [items] = useLive(() => api.recentActivity(projectId, 100), [projectId])
  return (
    <div className="page">
      <ActivityList items={items ?? []} />
    </div>
  )
}
