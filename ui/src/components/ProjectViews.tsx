// Secondary project tabs: notes, files and activity.
import { ArchiveRestore, Paperclip, Trash2 } from 'lucide-react'
import { useContextMenus, useEntityActions } from './actions'
import { api, relativeTime } from '../api'
import { useApp, useLive } from '../data'
import { ActivityList } from './Home'
import { Notespace } from './Notespace'
import type { ProjectSummary } from '../api'
import { FileTile } from './TaskPanel'

export function NotesView({ projectId, projects, selectedId, onSelect }: { projectId: number; projects: ProjectSummary[]; selectedId?: number | null; onSelect?: (id: number | null) => void }) {
  return (
    <div className="page notes-page">
      <Notespace projectId={projectId} projects={projects} selectedId={selectedId} onSelect={onSelect} />
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
  const { archivedTaskMenu } = useContextMenus()
  const { deleteTask } = useEntityActions()
  const [tasks] = useLive(() => api.listArchivedTasks(projectId), [projectId])
  return (
    <div className="page">
      <div className="section-title" style={{ marginBottom: 10 }}>
        {t('archive.tasks')}
      </div>
      {tasks && tasks.length > 0 ? (
        <div className="archive-list">
          {tasks.map((task) => (
            <div className="archive-row" key={task.id} onContextMenu={archivedTaskMenu(task)}>
              <span className="mono dim">#{task.id}</span>
              <span className="archive-title">{task.title}</span>
              <span className="dim">{task.archived_at ? relativeTime(task.archived_at, t, locale) : ''}</span>
              <button
                className="btn"
                onClick={() => run(api.archiveTask(task.id, false), t('archive.restored', { name: task.title }))}
              >
                <ArchiveRestore size={14} /> {t('archive.restore')}
              </button>
              <button className="btn danger-ghost" onClick={() => deleteTask(task)} aria-label={t('menu.delete')}>
                <Trash2 size={14} />
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
