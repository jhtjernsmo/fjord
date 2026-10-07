import type { ProjectSummary } from '../api'
import { useApp } from '../data'
import { displayKeys } from '../keymap'

interface Props {
  projects: ProjectSummary[]
  activeId: number | null
  onHome: () => void
  onSelect: (id: number) => void
  onNewProject: () => void
  onPalette: () => void
  onHelp: () => void
}

export function ProgressRing({ done, total, color }: { done: number; total: number; color: string }) {
  const percent = total === 0 ? 0 : Math.round((done / total) * 100)
  return <span className="ring" style={{ ['--p' as string]: percent, ['--c' as string]: color }} title={`${percent} %`} />
}

export function Sidebar({ projects, activeId, onHome, onSelect, onNewProject, onPalette, onHelp }: Props) {
  const { keymap } = useApp()
  const keys = (id: string) => displayKeys(keymap.bindings[id], keymap.leader)
  return (
    <aside className="sidebar">
      <div className="brand">
        <span className="brand-mark">◆</span> Fjord
      </div>
      <button className={`nav-item ${activeId === null ? 'active' : ''}`} onClick={onHome}>
        <span>🏠</span>
        <span className="name">Oversikt</span>
        <kbd>{keys('go.home')}</kbd>
      </button>
      <button className="nav-item" onClick={onPalette}>
        <span>⌘</span>
        <span className="name">Kommandoer</span>
        <kbd>{keys('palette.open')}</kbd>
      </button>

      <div className="nav-section">
        Prosjekter
        <button onClick={onNewProject} title="Nytt prosjekt" aria-label="Nytt prosjekt">
          +
        </button>
      </div>
      <nav className="project-list">
        {projects.map((p) => (
          <button
            key={p.id}
            className={`nav-item ${p.id === activeId ? 'active' : ''}`}
            onClick={() => onSelect(p.id)}
            title={p.description || p.name}
          >
            <span>{p.icon}</span>
            <span className="name">{p.name}</span>
            {p.overdue_count > 0 && <span className="chip overdue">{p.overdue_count}</span>}
            <ProgressRing done={p.done_count} total={p.task_count} color={p.color} />
          </button>
        ))}
        {projects.length === 0 && <div className="nav-item">Ingen prosjekter ennå</div>}
      </nav>

      <div className="sidebar-footer">
        <span>
          Leder-tast <kbd>{displayKeys('<leader>', keymap.leader)}</kbd>
        </span>
        <button className="nav-item" onClick={onHelp} style={{ padding: '4px 0' }}>
          <span className="name">Alle hurtigtaster</span>
          <kbd>{keys('help.toggle')}</kbd>
        </button>
      </div>
    </aside>
  )
}
