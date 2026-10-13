// Moves a task to another project. Subtasks, files and its analysis go with it.
import { useEffect, useState } from 'react'
import { api } from '../api'
import type { Status, Task } from '../api'
import { useApp, useLive } from '../data'

export function MoveTaskDialog({ task, onClose, onMoved }: { task: Task; onClose: () => void; onMoved?: () => void }) {
  const { t, run } = useApp()
  const [projects] = useLive(() => api.listProjects(false), [])
  const others = (projects ?? []).filter((p) => p.id !== task.project_id)
  const [projectId, setProjectId] = useState<number | null>(null)
  const [statuses, setStatuses] = useState<Status[]>([])
  const [statusId, setStatusId] = useState<number | null>(null)
  const [busy, setBusy] = useState(false)
  const target = others.find((p) => p.id === projectId) ?? others[0]
  const targetId = target?.id

  useEffect(() => {
    if (targetId === undefined) return
    let cancelled = false
    api
      .getBoard(targetId)
      .then((b) => !cancelled && setStatuses(b.statuses))
      .catch(() => !cancelled && setStatuses([]))
    return () => {
      cancelled = true
    }
  }, [targetId])

  const pickProject = (id: number) => {
    setProjectId(id)
    setStatusId(null)
  }

  const move = async () => {
    if (!target) return
    setBusy(true)
    const moved = await run(api.moveTaskToProject(task.id, target.id, statusId), t('moveProject.done', { name: target.name }))
    setBusy(false)
    if (moved === undefined) return
    onClose()
    onMoved?.()
  }

  return (
    <div className="overlay" onMouseDown={onClose}>
      <form
        className="dialog"
        role="dialog"
        aria-label={t('moveProject.title')}
        onMouseDown={(e) => e.stopPropagation()}
        onKeyDown={(e) => e.key === 'Escape' && onClose()}
        onSubmit={(e) => {
          e.preventDefault()
          move()
        }}
      >
        <h2>{t('moveProject.title')}</h2>
        <p className="hint">
          <span className="mono">#{task.id}</span> {task.title}
        </p>
        {projects && others.length === 0 ? (
          <p>{t('moveProject.none')}</p>
        ) : (
          <>
            <label className="row">
              {t('moveProject.project')}
              <select className="input" autoFocus value={target?.id ?? ''} onChange={(e) => pickProject(Number(e.target.value))}>
                {others.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
            </label>
            <label className="row">
              {t('task.status')}
              <select className="input" value={statusId ?? ''} onChange={(e) => setStatusId(e.target.value ? Number(e.target.value) : null)}>
                <option value="">{t('moveProject.sameColumn')}</option>
                {statuses.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.name}
                  </option>
                ))}
              </select>
            </label>
            <p className="hint">{t('moveProject.hint')}</p>
          </>
        )}
        <div className="actions">
          <button type="button" className="btn ghost" onClick={onClose}>
            {t('dialog.cancel')}
          </button>
          <button type="submit" className="btn primary" disabled={!target || busy}>
            {t('moveProject.move')}
          </button>
        </div>
      </form>
    </div>
  )
}
