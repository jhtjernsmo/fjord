// Overview on standup days: a draft of what you did since the last standup and what's next, ready to paste.
import { useState } from 'react'
import { Copy } from 'lucide-react'
import { api } from '../api'
import type { ProjectSummary, Status, Task } from '../api'
import { useApp, useLive } from '../data'
import { dateLocale } from '../i18n'
import { useNav } from '../nav'
import { buildStandup, previousStandup, standupText } from '../standup'
import type { StandupDraft, Weekday } from '../standup'
import { ProjectGlyph } from './Icons'

/** Enough history for a week between standups. */
const ACTIVITY_LIMIT = 500

async function loadDraft(projects: ProjectSummary[], since: Date, today: Date): Promise<StandupDraft> {
  const [activity, actor, boards] = await Promise.all([
    api.recentActivity(null, ACTIVITY_LIMIT),
    api.actor(),
    Promise.all(projects.map((p) => api.getBoard(p.id))),
  ])
  const tasks: Task[] = boards.flatMap((b) => b.tasks)
  const statuses: Status[] = boards.flatMap((b) => b.statuses)
  return buildStandup(activity, tasks, statuses, since, today, actor)
}

function StandupRow({ task, project, note }: { task: Task; project?: ProjectSummary; note?: string }) {
  const nav = useNav()
  return (
    <button className="standup-row" data-home-item data-home-action="open" onClick={() => nav.openTask(task.project_id, task.id)}>
      <span className="mono dim">#{task.id}</span>
      <span className="standup-title">{task.title}</span>
      <span className={`standup-note ${note ? '' : 'empty'}`}>{note}</span>
      <span className="due-project">
        {project && (
          <>
            <ProjectGlyph glyph={project.icon} color={project.color} />
            <span className="due-project-name">{project.name}</span>
          </>
        )}
      </span>
    </button>
  )
}

export function Standup({ projects, days }: { projects: ProjectSummary[]; days: Weekday[] }) {
  const { t, locale, toast } = useApp()
  const [today] = useState(() => new Date())
  const since = previousStandup(today, days)
  const projectIds = projects.map((p) => p.id).join()
  const [draft] = useLive(() => loadDraft(projects, since, today), [projectIds, days.join(), today])
  if (!draft) return null
  const sinceLabel = t('standup.since', { day: since.toLocaleDateString(dateLocale(locale), { weekday: 'long' }) })
  const project = (task: Task) => projects.find((p) => p.id === task.project_id)
  const copy = () =>
    navigator.clipboard
      ?.writeText(standupText(draft, { since: sinceLabel, done: t('standup.done'), today: t('standup.today'), dueToday: t('standup.dueToday'), overdue: t('standup.overdue'), blockers: t('standup.blockers'), none: t('standup.none') }))
      .then(() => toast(t('standup.copied'), 'success'))
  const nothingSince = draft.done.length + draft.workedOn.length === 0
  return (
    <div>
      <div className="section-title" style={{ marginBottom: 8 }}>
        <span>{t('standup.title')}</span>
        <button onClick={copy} title={t('standup.copyHint')}>
          <Copy size={12} /> {t('standup.copy')}
        </button>
      </div>
      <div className="standup">
        <div className="standup-group">{sinceLabel}</div>
        {draft.done.map((task) => (
          <StandupRow key={task.id} task={task} project={project(task)} note={t('standup.done')} />
        ))}
        {draft.workedOn.map((task) => (
          <StandupRow key={task.id} task={task} project={project(task)} />
        ))}
        {nothingSince && <div className="standup-empty">{t('standup.none')}</div>}
        <div className="standup-group">{t('standup.today')}</div>
        {draft.today.map(({ task, due }) => (
          <StandupRow key={task.id} task={task} project={project(task)} note={due === 'overdue' ? t('standup.overdue') : due === 'today' ? t('standup.dueToday') : undefined} />
        ))}
        {draft.today.length === 0 && <div className="standup-empty">{t('standup.none')}</div>}
      </div>
    </div>
  )
}
