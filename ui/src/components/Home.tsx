// Overview: greeting, stats, project cards and activity across all projects.
import { motion } from 'motion/react'
import { api, relativeTime } from '../api'
import type { Activity, ProjectSummary } from '../api'
import { useLive } from '../data'

function greeting(): string {
  const h = new Date().getHours()
  if (h < 5) return 'God natt'
  if (h < 10) return 'God morgen'
  if (h < 18) return 'God dag'
  return 'God kveld'
}

export function ActivityList({ items, projects }: { items: Activity[]; projects?: ProjectSummary[] }) {
  if (items.length === 0) return <div className="empty-state">Ingen aktivitet ennå.</div>
  return (
    <div className="activity">
      {items.map((a) => {
        const project = projects?.find((p) => p.id === a.project_id)
        return (
          <div className="act" key={a.id}>
            <span className={`who ${a.actor === 'claude' ? 'agent' : ''}`}>{a.actor === 'claude' ? '🤖 claude' : a.actor}</span>
            <span className="what">
              {a.summary}
              {project && <> · {project.icon} {project.name}</>}
            </span>
            <span className="when">{relativeTime(a.created_at)}</span>
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
  const [activity] = useLive(() => api.recentActivity(null, 15), [])
  const open = projects.reduce((n, p) => n + p.task_count - p.done_count, 0)
  const done = projects.reduce((n, p) => n + p.done_count, 0)
  const overdue = projects.reduce((n, p) => n + p.overdue_count, 0)
  const name = actor ? actor.charAt(0).toUpperCase() + actor.slice(1) : ''

  return (
    <div className="home">
      <div className="hello">
        <h2>
          {greeting()}, {name} 👋
        </h2>
        <p>{open === 0 ? 'Ingen åpne oppgaver. Nyt det!' : `Du har ${open} åpne oppgaver fordelt på ${projects.length} prosjekter.`}</p>
      </div>

      <div className="stats">
        <div className="stat">
          <div className="n">{projects.length}</div>
          <div className="l">Aktive prosjekter</div>
        </div>
        <div className="stat">
          <div className="n">{open}</div>
          <div className="l">Åpne oppgaver</div>
        </div>
        <div className="stat">
          <div className="n">{done}</div>
          <div className="l">Fullført</div>
        </div>
        <div className={`stat ${overdue > 0 ? 'alert' : ''}`}>
          <div className="n">{overdue}</div>
          <div className="l">Forfalt</div>
        </div>
      </div>

      {projects.length === 0 ? (
        <div className="empty-state">
          <span className="big">🏔️</span>
          <div>Velkommen til Fjord! Lag ditt første prosjekt for å komme i gang.</div>
          <button className="btn primary" onClick={onNewProject}>
            + Nytt prosjekt
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
                  <span className="icon">{p.icon}</span>
                  {p.name}
                </div>
                <div className="d">{p.description}</div>
                <div className="bar">
                  <div style={{ width: `${pct}%` }} />
                </div>
                <div className="foot">
                  <span>
                    {p.done_count}/{p.task_count} ferdig
                  </span>
                  <span>{p.overdue_count > 0 ? `⚠ ${p.overdue_count} forfalt` : `${pct} %`}</span>
                </div>
              </motion.button>
            )
          })}
        </div>
      )}

      <div>
        <div className="section-title" style={{ marginBottom: 8 }}>
          Siste aktivitet
        </div>
        <ActivityList items={activity ?? []} projects={projects} />
      </div>
    </div>
  )
}
