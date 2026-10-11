// Overview on standup days: one line with what you did since the last standup, ready to copy or expand.
import { useState } from 'react'
import { ChevronDown, ChevronRight, Copy } from 'lucide-react'
import { api } from '../api'
import type { ProjectSummary, Task } from '../api'
import { useApp, useLive } from '../data'
import { dateLocale } from '../i18n'
import { useNav } from '../nav'
import { buildStandup, previousStandup, standupText } from '../standup'
import type { StandupDraft, Weekday } from '../standup'

/** Enough history for a week between standups. */
const ACTIVITY_LIMIT = 500

async function loadDraft(projects: ProjectSummary[], since: Date): Promise<StandupDraft> {
  const [activity, actor, boards] = await Promise.all([
    api.recentActivity(null, ACTIVITY_LIMIT),
    api.actor(),
    Promise.all(projects.map((p) => api.getBoard(p.id))),
  ])
  return buildStandup(activity, boards.flatMap((b) => b.tasks), boards.flatMap((b) => b.statuses), since, actor)
}

function StandupList({ title, tasks, done = [] }: { title: string; tasks: Task[]; done?: Task[] }) {
  const { t } = useApp()
  const nav = useNav()
  const item = (task: Task, isDone: boolean) => (
    <li key={task.id}>
      <button data-home-item data-home-action="open" onClick={() => nav.openTask(task.project_id, task.id)}>
        <span className="mono dim">#{task.id}</span> {task.title}
        {isDone && <span className="standup-done"> {t('standup.done')}</span>}
      </button>
    </li>
  )
  return (
    <div className="standup-col">
      <div className="standup-col-title">{title}</div>
      <ul>
        {done.map((task) => item(task, true))}
        {tasks.map((task) => item(task, false))}
        {done.length + tasks.length === 0 && <li className="dim">{t('standup.none')}</li>}
      </ul>
    </div>
  )
}

export function Standup({ projects, days }: { projects: ProjectSummary[]; days: Weekday[] }) {
  const { t, locale, toast } = useApp()
  const [open, setOpen] = useState(false)
  const [today] = useState(() => new Date())
  const since = previousStandup(today, days)
  const projectIds = projects.map((p) => p.id).join()
  const [draft] = useLive(() => loadDraft(projects, since), [projectIds, days.join(), today])
  if (!draft) return null
  const sinceLabel = t('standup.since', { day: since.toLocaleDateString(dateLocale(locale), { weekday: 'long' }) })
  const copy = () =>
    navigator.clipboard
      ?.writeText(standupText(draft, { since: sinceLabel, done: t('standup.done'), today: t('standup.today'), blockers: t('standup.blockers'), none: t('standup.none') }))
      .then(() => toast(t('standup.copied'), 'success'))
  const summary = t('standup.summary', { done: draft.done.length, worked: draft.workedOn.length, today: draft.today.length })
  return (
    <div className="standup">
      <div className="standup-bar">
        <button className="standup-toggle" onClick={() => setOpen((v) => !v)} aria-expanded={open}>
          {open ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          <span className="standup-name">{t('standup.title')}</span>
          <span className="standup-summary">{summary}</span>
        </button>
        <button className="btn" onClick={copy} title={t('standup.copyHint')}>
          <Copy size={13} /> {t('standup.copy')}
        </button>
      </div>
      {open && (
        <div className="standup-body">
          <StandupList title={sinceLabel} done={draft.done} tasks={draft.workedOn} />
          <StandupList title={t('standup.today')} tasks={draft.today} />
        </div>
      )}
    </div>
  )
}
