// Overview: tasks waiting on someone else (e.g. a reply from the client), the longest waiting first.
import { useState } from 'react'
import { Hourglass } from 'lucide-react'
import { api } from '../api'
import type { ProjectSummary, Task } from '../api'
import { useApp, useLive } from '../data'
import { dateLocale } from '../i18n'
import { useNav } from '../nav'
import { waitedFor } from '../pending'
import { ProjectGlyph } from './Icons'

/** The Overview shows the first few; "Show all" expands the list. */
const SHOWN = 5

function PendingRow({ task, project }: { task: Task; project?: ProjectSummary }) {
  const { t, locale } = useApp()
  const nav = useNav()
  const since = task.pending_since ?? ''
  return (
    <button className="due-row pending-row" data-home-item data-home-action="open" onClick={() => nav.openTask(task.project_id, task.id)}>
      <span className="due-when">
        <Hourglass size={12} strokeWidth={1.8} /> {waitedFor(since, t)}
      </span>
      <span className="due-title">{task.title}</span>
      <span className="due-prio" />
      <span className="due-project">
        {project && (
          <>
            <ProjectGlyph glyph={project.icon} color={project.color} />
            <span className="due-project-name">{project.name}</span>
          </>
        )}
      </span>
      <span className="due-date">{new Date(since).toLocaleDateString(dateLocale(locale), { day: 'numeric', month: 'short' })}</span>
    </button>
  )
}

/** Renders nothing when no task is pending. */
export function PendingTasks({ projects }: { projects: ProjectSummary[] }) {
  const { t } = useApp()
  const [showAll, setShowAll] = useState(false)
  const [tasks] = useLive(() => api.listPendingTasks(), [])
  if (!tasks || tasks.length === 0) return null
  return (
    <div>
      <div className="section-title" style={{ marginBottom: 8 }}>
        <span className="mentions-heading">
          {t('pending.title')} <span className="count">{tasks.length}</span>
        </span>
      </div>
      <div className="due-list">
        {(showAll ? tasks : tasks.slice(0, SHOWN)).map((task) => (
          <PendingRow key={task.id} task={task} project={projects.find((p) => p.id === task.project_id)} />
        ))}
      </div>
      {tasks.length > SHOWN && (
        <button className="show-more" onClick={() => setShowAll((v) => !v)}>
          {showAll ? t('home.showLess') : t('mentions.showAll', { n: tasks.length })}
        </button>
      )}
    </div>
  )
}
