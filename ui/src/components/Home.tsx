// Overview: today's date, what's due, mentions, projects and activity across all projects.
import { ArchiveRestore, Plus } from 'lucide-react'
import { api, describeActivity, relativeTime } from '../api'
import type { Activity, ProjectSummary } from '../api'
import { useActions, useApp, useLive } from '../data'
import { dateLocale } from '../i18n'
import { AgentTag, ProjectGlyph } from './Icons'
import { useContextMenus } from './actions'
import { MentionsSection } from './Mentions'
import { DueThisWeek } from './DueThisWeek'
import { useRef, useState } from 'react'

/** Overview shows a short activity preview; "Show all" loads more. */
const ACTIVITY_PREVIEW = 8
const ACTIVITY_ALL = 50

export function ActivityList({ items, projects }: { items: Activity[]; projects?: ProjectSummary[] }) {
  const { t, locale } = useApp()
  if (items.length === 0) return <div className="empty-state">{t('activity.none')}</div>
  return (
    <div className="activity">
      {items.map((a) => {
        const project = projects?.find((p) => p.id === a.project_id)
        return (
          <div className="act" key={a.id}>
            <span className="who">{a.actor === 'claude' ? <AgentTag /> : a.actor}</span>
            <span className="what">
              {describeActivity(a, t)}
              {project && <span className="act-project"> · {project.name}</span>}
            </span>
            <span className="when">{relativeTime(a.created_at, t, locale)}</span>
          </div>
        )
      })}
    </div>
  )
}

interface Props {
  projects: ProjectSummary[]
  onOpen: (id: number) => void
  onNewProject: () => void
  /** Off while a dialog or the palette is open. */
  keysEnabled: boolean
}

/**
 * Keyboard navigation over the Overview's rows (due tasks, mentions, project cards).
 * Rows mark themselves with `data-home-item`, and their buttons with
 * `data-home-action`, so each section keeps its own data and layout.
 */
function useHomeKeys(enabled: boolean) {
  const ref = useRef<HTMLDivElement>(null)
  const items = () => [...(ref.current?.querySelectorAll<HTMLElement>('[data-home-item]') ?? [])]
  const current = () => (document.activeElement as HTMLElement | null)?.closest<HTMLElement>('[data-home-item]') ?? null
  const focusAt = (index: number) => {
    const list = items()
    const el = list[Math.min(Math.max(index, 0), list.length - 1)]
    el?.focus()
    el?.scrollIntoView({ block: 'nearest' })
  }
  const step = (delta: number) => {
    const i = items().indexOf(current() as HTMLElement)
    focusAt(i < 0 ? (delta > 0 ? 0 : items().length - 1) : i + delta)
  }
  const act = (action: string) => {
    const row = current()
    if (!row) return
    const selector = `[data-home-action="${action}"]`
    const target = row.matches(selector) ? row : row.querySelector<HTMLElement>(selector)
    if (!target) return
    const index = items().indexOf(row)
    target.click()
    // Dismissing removes the row; keep the cursor on the row that took its place.
    if (action === 'dismiss') window.setTimeout(() => focusAt(index))
  }
  useActions(
    enabled
      ? {
          'home.next': () => step(1),
          'home.prev': () => step(-1),
          'home.open': () => act('open'),
          'home.addTask': () => act('add'),
          'home.dismiss': () => act('dismiss'),
        }
      : {},
    'home',
  )
  return ref
}

export function Home({ projects, onOpen, onNewProject, keysEnabled }: Props) {
  const { t, run, locale } = useApp()
  const homeRef = useHomeKeys(keysEnabled)
  const [showAll, setShowAll] = useState(false)
  const [activity] = useLive(() => api.recentActivity(null, showAll ? ACTIVITY_ALL : ACTIVITY_PREVIEW + 1), [showAll])
  const { projectMenu } = useContextMenus()
  const [allProjects] = useLive(() => api.listProjects(true), [])
  const archived = (allProjects ?? []).filter((p) => p.archived_at)
  const open = projects.reduce((n, p) => n + p.task_count - p.done_count, 0)
  const overdue = projects.reduce((n, p) => n + p.overdue_count, 0)
  const [today] = useState(() => new Date())
  const date = today.toLocaleDateString(dateLocale(locale), { weekday: 'long', day: 'numeric', month: 'long' })

  return (
    <div className="home" ref={homeRef}>
      <div className="hello">
        <h2>{date.charAt(0).toUpperCase() + date.slice(1)}</h2>
        <p>
          {open === 0 ? t('home.noOpen') : t('home.open', { open, projects: projects.length })}
          {overdue > 0 && (
            <>
              {', '}
              <span className="overdue-text">{t('home.overdueCount', { n: overdue })}</span>
            </>
          )}
        </p>
      </div>

      <DueThisWeek projects={projects} />

      <MentionsSection projects={projects} />

      {projects.length === 0 ? (
        <div className="empty-state">
          <pre className="ascii-logo">{'  /\\    /\\\n /  \\  /  \\\n/    \\/    \\  fjord'}</pre>
          <div>{t('home.welcome')}</div>
          <button className="btn primary" onClick={onNewProject}>
            <Plus size={14} /> {t('app.newProject')}
          </button>
        </div>
      ) : (
        <div>
          <div className="section-title" style={{ marginBottom: 8 }}>
            {t('app.projects')}
          </div>
          <div className="project-list">
            {projects.map((p) => {
              const pct = p.task_count === 0 ? 0 : Math.round((p.done_count / p.task_count) * 100)
              return (
                <button
                  key={p.id}
                  className="project-row"
                  data-home-item
                  data-home-action="open"
                  style={{ ['--c' as string]: p.color }}
                  onClick={() => onOpen(p.id)}
                  onContextMenu={projectMenu(p, { onOpen: () => onOpen(p.id) })}
                >
                  <ProjectGlyph glyph={p.icon} color={p.color} />
                  <span className="project-row-name">{p.name}</span>
                  <span className="project-row-desc">{p.description}</span>
                  <span className="bar">
                    <div style={{ width: `${pct}%` }} />
                  </span>
                  <span className="project-row-count">
                    {p.done_count}/{p.task_count}
                  </span>
                  <span className={p.overdue_count > 0 ? 'overdue-text' : ''}>
                    {p.overdue_count > 0 ? t('home.overdueCount', { n: p.overdue_count }) : ''}
                  </span>
                </button>
              )
            })}
          </div>
        </div>
      )}

      <div>
        <div className="section-title" style={{ marginBottom: 8 }}>
          {t('home.recent')}
        </div>
        <ActivityList items={(activity ?? []).slice(0, showAll ? ACTIVITY_ALL : ACTIVITY_PREVIEW)} projects={projects} />
        {(showAll || (activity?.length ?? 0) > ACTIVITY_PREVIEW) && (
          <button className="show-more" onClick={() => setShowAll((v) => !v)}>
            {showAll ? t('home.showLess') : t('home.showAll')}
          </button>
        )}
      </div>

      {archived.length > 0 && (
        <div>
          <div className="section-title" style={{ marginBottom: 8 }}>
            {t('archive.projects')}
          </div>
          <div className="archive-list">
            {archived.map((p) => (
              <div className="archive-row" key={p.id} onContextMenu={projectMenu(p, { onOpen: () => undefined })}>
                <ProjectGlyph glyph={p.icon} color={p.color} />
                <span className="archive-title">{p.name}</span>
                <button className="btn" onClick={() => run(api.archiveProject(p.id, false), t('archive.restored', { name: p.name }))}>
                  <ArchiveRestore size={14} /> {t('archive.restore')}
                </button>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
