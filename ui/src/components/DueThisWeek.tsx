// Overview: open tasks due by Sunday across all projects, overdue ones first.
import { useState } from 'react'
import { CalendarDays } from 'lucide-react'
import { api } from '../api'
import type { ProjectSummary, Task } from '../api'
import { useApp, useLive } from '../data'
import { dueWhen, endOfWeek, parseDue } from '../dueWeek'
import { dateLocale } from '../i18n'
import { useNav } from '../nav'
import { ProjectGlyph } from './Icons'

/** The Overview shows the first few; "Show all" expands the list. */
const SHOWN = 5
const PRIORITY_ICON = ['', '↓', '→', '↑']

function DueRow({ task, project, today }: { task: Task; project?: ProjectSummary; today: Date }) {
  const { t, locale } = useApp()
  const nav = useNav()
  const due = task.due_at ?? ''
  const when = dueWhen(due, today)
  const date = parseDue(due)
  const label =
    when === 'overdue'
      ? t('due.overdue')
      : when === 'today'
        ? t('due.today')
        : when === 'tomorrow'
          ? t('due.tomorrow')
          : date.toLocaleDateString(dateLocale(locale), { weekday: 'long' })
  return (
    <button className={`due-row due-${when}`} data-home-item data-home-action="open" onClick={() => nav.openTask(task.project_id, task.id)}>
      <span className="due-when">{label}</span>
      <span className="due-title">{task.title}</span>
      {/* Every column is always rendered so rows line up in the grid. */}
      <span
        className={`due-prio prio-${task.priority}`}
        title={task.priority > 0 ? `${t('task.priority')}: ${t(`priority.${task.priority}` as 'priority.1')}` : undefined}
      >
        {PRIORITY_ICON[task.priority]}
      </span>
      <span className="due-project">
        {project && (
          <>
            <ProjectGlyph glyph={project.icon} color={project.color} />
            <span className="due-project-name">{project.name}</span>
          </>
        )}
      </span>
      <span className="due-date">{date.toLocaleDateString(dateLocale(locale), { day: 'numeric', month: 'short' })}</span>
    </button>
  )
}

/** Renders nothing when nothing is due this week. */
export function DueThisWeek({ projects }: { projects: ProjectSummary[] }) {
  const { t } = useApp()
  const [showAll, setShowAll] = useState(false)
  // Read once per visit to the Overview; the labels are relative to it.
  const [today] = useState(() => new Date())
  const [tasks] = useLive(() => api.listDueTasks(endOfWeek(today)), [today])
  if (!tasks || tasks.length === 0) return null
  return (
    <div>
      <div className="section-title" style={{ marginBottom: 8 }}>
        <span className="mentions-heading">
          <CalendarDays size={13} /> {t('due.title')} <span className="count">{tasks.length}</span>
        </span>
      </div>
      <div className="due-list">
        {(showAll ? tasks : tasks.slice(0, SHOWN)).map((task) => (
          <DueRow key={task.id} task={task} today={today} project={projects.find((p) => p.id === task.project_id)} />
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
