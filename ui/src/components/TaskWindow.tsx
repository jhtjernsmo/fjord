// A task popped out into its own window. Stays live with the main window (same database);
// links to notes, projects and other tasks open in the main window.
import { useCallback, useEffect, useMemo, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { Paperclip } from 'lucide-react'
import { api } from '../api'
import type { MainNav } from '../api'
import { useApp, useLive } from '../data'
import { NavContext } from '../nav'
import type { Nav } from '../nav'
import { TaskPanel } from './TaskPanel'

const closeWindow = () => void getCurrentWindow().close()

/** Files dropped on the window are attached to the task, as in the main window. */
function useFileDrop(projectId: number | undefined, taskId: number) {
  const { run, toast, t } = useApp()
  const [dropping, setDropping] = useState(false)
  useEffect(() => {
    if (projectId === undefined) return
    const unlisten = getCurrentWebview()
      .onDragDropEvent(async ({ payload }) => {
        if (payload.type === 'over' || payload.type === 'enter') setDropping(true)
        else if (payload.type === 'leave') setDropping(false)
        else if (payload.type === 'drop') {
          setDropping(false)
          const results = await run(api.attachFiles(projectId, taskId, payload.paths))
          const ok = results?.filter((r) => r.attachment).length ?? 0
          results?.filter((r) => r.error).forEach((r) => toast(`${r.path}: ${r.error}`, 'error'))
          if (ok > 0) toast(t('drop.attached', { n: ok }), 'success')
        }
      })
      .catch(() => undefined)
    return () => {
      unlisten.then((fn) => fn?.())
    }
  }, [projectId, taskId, run, toast, t])
  return dropping
}

export function TaskWindow({ taskId }: { taskId: number }) {
  const { t } = useApp()
  // null once the task is deleted; follows it if it moves to another project.
  const [board] = useLive(async () => {
    const task = await api.getTask(taskId).catch(() => null)
    return task && (await api.getBoard(task.project_id))
  }, [taskId])

  const nav = useMemo<Nav>(() => {
    const show = (n: MainNav) => void api.navigateMain(n).catch(() => undefined)
    return {
      openProject: (id) => show({ kind: 'project', id }),
      openTask: (projectId, id) => show({ kind: 'task', project_id: projectId, task_id: id }),
      openNote: (id, projectId) => show({ kind: 'note', id, project_id: projectId }),
      createNote: () => undefined,
      editProject: (project) => show({ kind: 'project', id: project.id }),
    }
  }, [])

  // Archived or moved: close only if the task itself is gone or archived.
  const onGone = useCallback(() => {
    api.getTask(taskId).then((task) => task.archived_at && closeWindow(), closeWindow)
  }, [taskId])

  const dropping = useFileDrop(board?.project.id, taskId)

  if (board === undefined) return null
  if (board === null) return <p className="task-window-gone">{t('taskWindow.gone')}</p>
  return (
    <NavContext.Provider value={nav}>
      <div className="task-window">
        <TaskPanel key={board.project.id} taskId={taskId} statuses={board.statuses} onClose={closeWindow} onGone={onGone} detached />
        {dropping && (
          <div className="drop-overlay">
            <Paperclip size={22} /> {t('drop.task')}
          </div>
        )}
      </div>
    </NavContext.Provider>
  )
}
