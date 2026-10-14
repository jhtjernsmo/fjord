// Makes a task a subtask of another task in the same project.
import { useState } from 'react'
import { api } from '../api'
import type { Task } from '../api'
import { useApp, useLive } from '../data'

const LIST_ROWS = 8

export function NestTaskDialog({ task, onClose }: { task: Task; onClose: () => void }) {
  const { t, run } = useApp()
  const [board] = useLive(() => api.getBoard(task.project_id), [task.project_id])
  const [query, setQuery] = useState('')
  const [picked, setPicked] = useState<number | null>(null)
  const [busy, setBusy] = useState(false)
  const q = query.trim().toLowerCase()
  const candidates = (board?.tasks ?? []).filter(
    (c) => c.id !== task.id && (c.parent_id ?? null) === null && (!q || c.title.toLowerCase().includes(q) || `#${c.id}`.includes(q)),
  )
  const parent = candidates.find((c) => c.id === picked) ?? candidates[0]

  // Arrow keys move through the list while you type.
  const step = (e: React.KeyboardEvent) => {
    const delta = e.key === 'ArrowDown' ? 1 : e.key === 'ArrowUp' ? -1 : 0
    if (!delta || candidates.length === 0) return
    e.preventDefault()
    const i = Math.max(0, candidates.findIndex((c) => c.id === parent?.id))
    setPicked(candidates[Math.min(candidates.length - 1, Math.max(0, i + delta))].id)
  }

  const nest = async () => {
    if (!parent) return
    setBusy(true)
    const done = await run(api.setTaskParent(task.id, parent.id), t('nest.done', { title: parent.title }))
    setBusy(false)
    if (done !== undefined) onClose()
  }

  return (
    <div className="overlay" onMouseDown={onClose}>
      <form
        className="dialog"
        role="dialog"
        aria-label={t('nest.title')}
        onMouseDown={(e) => e.stopPropagation()}
        onKeyDown={(e) => e.key === 'Escape' && onClose()}
        onSubmit={(e) => {
          e.preventDefault()
          nest()
        }}
      >
        <h2>{t('nest.title')}</h2>
        <p className="hint">
          <span className="mono">#{task.id}</span> {task.title}
        </p>
        <label className="row">
          {t('nest.parent')}
          <input className="input" autoFocus value={query} onChange={(e) => setQuery(e.target.value)} onKeyDown={step} placeholder={t('nest.search')} />
        </label>
        {board && candidates.length === 0 ? (
          <p>{t(q ? 'nest.noMatch' : 'nest.none')}</p>
        ) : (
          <select
            className="input nest-list"
            size={LIST_ROWS}
            aria-label={t('nest.parent')}
            value={parent?.id ?? ''}
            onChange={(e) => setPicked(Number(e.target.value))}
            onDoubleClick={nest}
          >
            {candidates.map((c) => (
              <option key={c.id} value={c.id}>
                #{c.id} {c.title}
              </option>
            ))}
          </select>
        )}
        <div className="actions">
          <button type="button" className="btn ghost" onClick={onClose}>
            {t('dialog.cancel')}
          </button>
          <button type="submit" className="btn primary" disabled={!parent || busy}>
            {t('nest.confirm')}
          </button>
        </div>
      </form>
    </div>
  )
}
