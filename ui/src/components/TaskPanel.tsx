// Slide-in task details: title, status, priority, due date, markdown, files.
import { useEffect, useState } from 'react'
import Markdown from 'react-markdown'
import { motion } from 'motion/react'
import { api, formatBytes, PRIORITIES, relativeTime } from '../api'
import type { Attachment, Status } from '../api'
import { useApp, useLive } from '../data'

const FILE_ICONS: [RegExp, string][] = [
  [/\.(pdf)$/i, '📕'],
  [/\.(png|jpe?g|gif|webp|svg)$/i, '🖼️'],
  [/\.(zip|tar|gz|7z|rar)$/i, '🗜️'],
  [/\.(mp4|mov|webm|mkv)$/i, '🎬'],
  [/\.(mp3|wav|flac|ogg)$/i, '🎵'],
  [/\.(md|txt|rtf)$/i, '📝'],
  [/\.(js|ts|tsx|rs|py|swift|kt|java|json|html|css)$/i, '💻'],
]

export function fileIcon(name: string): string {
  return FILE_ICONS.find(([re]) => re.test(name))?.[1] ?? '📄'
}

export function FileTile({ file, onDetach }: { file: Attachment; onDetach?: () => void }) {
  const { run } = useApp()
  const [preview, setPreview] = useState<string | null>(null)
  useEffect(() => {
    let cancelled = false
    api.previewAttachment(file.id).then((url) => !cancelled && setPreview(url)).catch(() => undefined)
    return () => {
      cancelled = true
    }
  }, [file.id])
  return (
    <div className="file">
      <button className="thumb" onClick={() => run(api.openAttachment(file.id))} title="Åpne">
        {preview ? <img src={preview} alt={file.original_name} /> : fileIcon(file.original_name)}
      </button>
      <div className="meta">
        <div className="fname" title={file.original_name}>
          {file.original_name}
        </div>
        <div className="sub">
          <span>
            {formatBytes(file.size)} · {file.added_by === 'claude' ? '🤖' : file.added_by}
          </span>
          {onDetach && (
            <button onClick={onDetach} title="Fjern fra oppgaven" aria-label="Fjern fil">
              ✕
            </button>
          )}
        </div>
      </div>
    </div>
  )
}

interface Props {
  taskId: number
  statuses: Status[]
  onClose: () => void
}

export function TaskPanel({ taskId, statuses, onClose }: Props) {
  const { run } = useApp()
  const [board] = useLive(async () => {
    const all = await api.getBoard(statuses[0].project_id)
    return all.tasks.find((t) => t.id === taskId) ?? null
  }, [taskId])
  const [files] = useLive(() => api.listAttachments(statuses[0].project_id, taskId), [taskId])
  const task = board ?? null
  const [title, setTitle] = useState('')
  const [body, setBody] = useState('')
  const [editing, setEditing] = useState(false)

  useEffect(() => {
    if (!task) return
    setTitle(task.title)
    if (!editing) setBody(task.body_md)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [task?.id, task?.title, task?.body_md])

  if (board === null) {
    return null
  }
  if (!task) return <motion.aside className="panel" initial={{ x: 40, opacity: 0 }} animate={{ x: 0, opacity: 1 }} />

  const save = (patch: Parameters<typeof api.updateTask>[1]) => run(api.updateTask(task.id, patch))
  const status = statuses.find((s) => s.id === task.status_id)

  return (
    <>
      <div className="scrim" onClick={onClose} />
      <motion.aside
        className="panel"
        initial={{ x: 60, opacity: 0 }}
        animate={{ x: 0, opacity: 1 }}
        transition={{ type: 'spring', stiffness: 420, damping: 36 }}
        aria-label="Oppgavedetaljer"
      >
        <div className="panel-head">
          <span>#{task.id}</span>
          <span>
            · laget av {task.created_by === 'claude' ? '🤖 claude' : task.created_by} · {relativeTime(task.created_at)}
          </span>
          <span className="spacer" />
          <button className="btn ghost" onClick={() => run(api.archiveTask(task.id, true), 'Oppgaven er arkivert').then(onClose)}>
            Arkiver
          </button>
          <button className="btn ghost" onClick={onClose} aria-label="Lukk">
            ✕ <kbd>Esc</kbd>
          </button>
        </div>
        <div className="panel-body">
          <input
            className="title-input"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            onBlur={() => title.trim() && title !== task.title && save({ title })}
            onKeyDown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
            aria-label="Tittel"
          />

          <div className="fields">
            <label>Status</label>
            <select
              className="input"
              value={task.status_id}
              onChange={(e) => run(api.moveTask(task.id, Number(e.target.value)))}
            >
              {statuses.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                </option>
              ))}
            </select>

            <label>Prioritet</label>
            <div className="segmented" role="radiogroup" aria-label="Prioritet">
              {PRIORITIES.map((p, i) => (
                <button key={p} className={task.priority === i ? 'on' : ''} onClick={() => save({ priority: i })} role="radio" aria-checked={task.priority === i}>
                  {p}
                </button>
              ))}
            </div>

            <label>Frist</label>
            <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
              <input
                type="date"
                className="input"
                value={task.due_at ?? ''}
                onChange={(e) => save({ due_at: e.target.value || null })}
              />
              {task.due_at && (
                <button className="btn ghost" onClick={() => save({ due_at: null })}>
                  Fjern
                </button>
              )}
            </div>
          </div>

          <div>
            <div className="section-title">
              Beskrivelse
              <button onClick={() => (editing ? (save({ body_md: body }), setEditing(false)) : setEditing(true))}>
                {editing ? 'Lagre' : 'Rediger'}
              </button>
            </div>
            {editing ? (
              <textarea
                className="md-editor"
                autoFocus
                value={body}
                onChange={(e) => setBody(e.target.value)}
                onBlur={() => {
                  if (body !== task.body_md) save({ body_md: body })
                  setEditing(false)
                }}
                onKeyDown={(e) => e.key === 'Escape' && (e.stopPropagation(), e.currentTarget.blur())}
                placeholder="Markdown støttes: **fet**, - lister, `kode` …"
              />
            ) : (
              <div className={`markdown ${task.body_md ? '' : 'empty'}`} onClick={() => setEditing(true)}>
                {task.body_md ? <Markdown>{task.body_md}</Markdown> : 'Klikk for å legge til en beskrivelse'}
              </div>
            )}
          </div>

          <div>
            <div className="section-title">Filer ({files?.length ?? 0})</div>
            <div className="files" style={{ marginTop: 10 }}>
              {(files ?? []).map((f) => (
                <FileTile key={f.id} file={f} onDetach={() => run(api.detachFile(f.id))} />
              ))}
            </div>
            <div className="dropzone" style={{ marginTop: 10 }}>
              Dra filer hit for å legge dem ved «{task.title}»
            </div>
          </div>

          {status?.is_done && <div className="chip" style={{ alignSelf: 'flex-start' }}>✅ Ferdig</div>}
        </div>
      </motion.aside>
    </>
  )
}
