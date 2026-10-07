// Secondary project tabs: notes, files and activity.
import { useEffect, useState } from 'react'
import Markdown from 'react-markdown'
import { ArchiveRestore, NotebookText, Paperclip, Plus } from 'lucide-react'
import { api, relativeTime } from '../api'
import type { Note } from '../api'
import { useApp, useLive } from '../data'
import { ActivityList } from './Home'
import { FileTile } from './TaskPanel'

export function NotesView({ projectId }: { projectId: number }) {
  const { run, t, locale } = useApp()
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
    const note = await run(api.addNote(projectId, t('notes.untitled'), ''))
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
            <Plus size={14} /> {t('notes.new').replace('+ ', '')}
          </button>
          {(notes ?? []).map((n) => (
            <button key={n.id} className={`nav-item ${n.id === active?.id ? 'active' : ''}`} onClick={() => setActiveId(n.id)}>
              <NotebookText size={15} strokeWidth={1.6} />
              <span className="name">{n.title}</span>
              <span className="note-time">{relativeTime(n.updated_at, t, locale)}</span>
            </button>
          ))}
        </div>
        {active ? (
          <div className="note-editor">
            <input className="title-input" value={title} onChange={(e) => setTitle(e.target.value)} onBlur={save} aria-label={t('task.title')} />
            <div className="section-title">
              {t('notes.content')}
              <button onClick={() => (editing ? (save(), setEditing(false)) : setEditing(true))}>
                {editing ? t('task.save') : t('task.edit')}
              </button>
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
                {active.body_md ? <Markdown>{active.body_md}</Markdown> : t('notes.empty')}
              </div>
            )}
          </div>
        ) : (
          <div className="empty-state">{t('notes.none')}</div>
        )}
      </div>
    </div>
  )
}

export function FilesView({ projectId }: { projectId: number }) {
  const { run, t } = useApp()
  const [files] = useLive(() => api.listAttachments(projectId), [projectId])
  return (
    <div className="page">
      <div className="dropzone" style={{ marginBottom: 16 }}>
        {t('files.dropProject')}
      </div>
      {files && files.length > 0 ? (
        <div className="files">
          {files.map((f) => (
            <FileTile key={f.id} file={f} onDetach={() => run(api.detachFile(f.id))} />
          ))}
        </div>
      ) : (
        <div className="empty-state">
          <Paperclip size={36} strokeWidth={1.2} />
          {t('files.none')}
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

export function ArchiveView({ projectId }: { projectId: number }) {
  const { run, t, locale } = useApp()
  const [tasks] = useLive(() => api.listArchivedTasks(projectId), [projectId])
  return (
    <div className="page">
      <div className="section-title" style={{ marginBottom: 10 }}>
        {t('archive.tasks')}
      </div>
      {tasks && tasks.length > 0 ? (
        <div className="archive-list">
          {tasks.map((task) => (
            <div className="archive-row" key={task.id}>
              <span className="mono dim">#{task.id}</span>
              <span className="archive-title">{task.title}</span>
              <span className="dim">{task.archived_at ? relativeTime(task.archived_at, t, locale) : ''}</span>
              <button
                className="btn"
                onClick={() => run(api.archiveTask(task.id, false), t('archive.restored', { name: task.title }))}
              >
                <ArchiveRestore size={14} /> {t('archive.restore')}
              </button>
            </div>
          ))}
        </div>
      ) : (
        <div className="empty-state">{t('archive.none')}</div>
      )}
    </div>
  )
}
