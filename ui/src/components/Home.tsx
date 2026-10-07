// Overview: greeting, stats, project cards and activity across all projects.
import { motion } from 'motion/react'
import { ArchiveRestore, Plus } from 'lucide-react'
import { api, describeActivity, relativeTime } from '../api'
import type { Activity, ProjectSummary } from '../api'
import { useApp, useLive } from '../data'
import type { MessageKey } from '../i18n'
import { AgentTag, ProjectGlyph } from './Icons'

function greetingKey(): MessageKey {
  const h = new Date().getHours()
  if (h < 5) return 'greeting.night'
  if (h < 10) return 'greeting.morning'
  if (h < 18) return 'greeting.day'
  return 'greeting.evening'
}

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
  actor: string
  projects: ProjectSummary[]
  onOpen: (id: number) => void
  onNewProject: () => void
}

export function Home({ actor, projects, onOpen, onNewProject }: Props) {
  const { t, run } = useApp()
  const [activity] = useLive(() => api.recentActivity(null, 15), [])
  const [allProjects] = useLive(() => api.listProjects(true), [])
  const archived = (allProjects ?? []).filter((p) => p.archived_at)
  const open = projects.reduce((n, p) => n + p.task_count - p.done_count, 0)
  const done = projects.reduce((n, p) => n + p.done_count, 0)
  const overdue = projects.reduce((n, p) => n + p.overdue_count, 0)
  const name = actor ? actor.charAt(0).toUpperCase() + actor.slice(1) : ''

  return (
    <div className="home">
      <div className="hello">
        <h2>
          {t(greetingKey())}, {name}
        </h2>
        <p>{open === 0 ? t('home.noOpen') : t('home.open', { open, projects: projects.length })}</p>
      </div>

      <div className="stats">
        <div className="stat">
          <div className="n">{projects.length}</div>
          <div className="l">{t('home.activeProjects')}</div>
        </div>
        <div className="stat">
          <div className="n">{open}</div>
          <div className="l">{t('home.openTasks')}</div>
        </div>
        <div className="stat">
          <div className="n">{done}</div>
          <div className="l">{t('home.completed')}</div>
        </div>
        <div className={`stat ${overdue > 0 ? 'alert' : ''}`}>
          <div className="n">{overdue}</div>
          <div className="l">{t('home.overdue')}</div>
        </div>
      </div>

      {projects.length === 0 ? (
        <div className="empty-state">
          <pre className="ascii-logo">{'  /\\    /\\\n /  \\  /  \\\n/    \\/    \\  fjord'}</pre>
          <div>{t('home.welcome')}</div>
          <button className="btn primary" onClick={onNewProject}>
            <Plus size={14} /> {t('app.newProject')}
          </button>
        </div>
      ) : (
        <div className="cards">
          {projects.map((p, i) => {
            const pct = p.task_count === 0 ? 0 : Math.round((p.done_count / p.task_count) * 100)
            return (
              <motion.button
                key={p.id}
                className="pcard"
                style={{ ['--c' as string]: p.color }}
                onClick={() => onOpen(p.id)}
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: i * 0.04 }}
              >
                <div className="top">
                  <ProjectGlyph glyph={p.icon} color={p.color} size="lg" />
                  {p.name}
                </div>
                <div className="d">{p.description}</div>
                <div className="bar">
                  <div style={{ width: `${pct}%` }} />
                </div>
                <div className="foot">
                  <span>{t('home.done', { done: p.done_count, total: p.task_count })}</span>
                  <span className={p.overdue_count > 0 ? 'overdue-text' : ''}>
                    {p.overdue_count > 0 ? `! ${t('home.overdueCount', { n: p.overdue_count })}` : `${pct} %`}
                  </span>
                </div>
              </motion.button>
            )
          })}
        </div>
      )}

      <div>
        <div className="section-title" style={{ marginBottom: 8 }}>
          {t('home.recent')}
        </div>
        <ActivityList items={activity ?? []} projects={projects} />
      </div>

      {archived.length > 0 && (
        <div>
          <div className="section-title" style={{ marginBottom: 8 }}>
            {t('archive.projects')}
          </div>
          <div className="archive-list">
            {archived.map((p) => (
              <div className="archive-row" key={p.id}>
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
