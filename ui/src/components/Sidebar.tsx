import { Bug, Command, House, Keyboard, Languages, NotebookText, Plus, Settings } from 'lucide-react'
import type { ProjectSummary } from '../api'
import { useApp } from '../data'
import { reportBug } from '../reportBug'
import { LOCALES } from '../i18n'
import { displayKeys } from '../keymap'
import { ProjectGlyph } from './Icons'
import iconUrl from '../assets/icon.svg'
import { useContextMenus } from './actions'

interface Props {
  projects: ProjectSummary[]
  activeId: number | null
  onHome: () => void
  onSelect: (id: number) => void
  onNewProject: () => void
  onPalette: () => void
  onHelp: () => void
  onSettings: () => void
  onNotes: () => void
  notesActive: boolean
}

export function ProgressRing({ done, total, color }: { done: number; total: number; color: string }) {
  const percent = total === 0 ? 0 : Math.round((done / total) * 100)
  return <span className="ring" style={{ ['--p' as string]: percent, ['--c' as string]: color }} title={`${percent} %`} />
}

export function Sidebar({ projects, activeId, onHome, onSelect, onNewProject, onPalette, onHelp, onSettings, onNotes, notesActive }: Props) {
  const { keymap, t, locale, setLocale } = useApp()
  const { projectMenu } = useContextMenus()
  const keys = (id: string) => displayKeys(keymap.bindings[id], keymap.leader)
  const nextLocale = LOCALES[(LOCALES.findIndex((l) => l.id === locale) + 1) % LOCALES.length]
  return (
    <aside className="sidebar">
      <div className="brand">
        <img className="brand-icon" src={iconUrl} alt="" width={28} height={28} /> fjord
      </div>
      <button className={`nav-item ${activeId === null && !notesActive ? 'active' : ''}`} onClick={onHome}>
        <House size={16} strokeWidth={1.6} />
        <span className="name">{t('app.overview')}</span>
        <kbd>{keys('go.home')}</kbd>
      </button>
      <button className={`nav-item ${notesActive ? 'active' : ''}`} onClick={onNotes}>
        <NotebookText size={16} strokeWidth={1.6} />
        <span className="name">{t('nav.notes')}</span>
        <kbd>{keys('go.notes')}</kbd>
      </button>
      <button className="nav-item" onClick={onPalette}>
        <Command size={16} strokeWidth={1.6} />
        <span className="name">{t('app.commands')}</span>
        <kbd>{keys('palette.open')}</kbd>
      </button>

      <div className="nav-section">
        {t('app.projects')}
        <button onClick={onNewProject} title={t('app.newProject')} aria-label={t('app.newProject')}>
          <Plus size={15} />
        </button>
      </div>
      <nav className="project-list">
        {projects.map((p) => (
          <button
            key={p.id}
            className={`nav-item ${p.id === activeId ? 'active' : ''}`}
            onClick={() => onSelect(p.id)}
            onContextMenu={projectMenu(p, { onOpen: () => onSelect(p.id), onGone: p.id === activeId ? onHome : undefined })}
            title={p.description || p.name}
          >
            <ProjectGlyph glyph={p.icon} color={p.color} />
            <span className="name">{p.name}</span>
            {p.overdue_count > 0 && <span className="chip overdue">{p.overdue_count}</span>}
            <ProgressRing done={p.done_count} total={p.task_count} color={p.color} />
          </button>
        ))}
        {projects.length === 0 && <div className="nav-item">{t('app.noProjects')}</div>}
      </nav>

      <div className="sidebar-footer">
        <button className="nav-item" onClick={onSettings}>
          <Settings size={15} strokeWidth={1.6} />
          <span className="name">{t('settings.title')}</span>
          <kbd>{keys('settings.open')}</kbd>
        </button>
        <button className="nav-item" onClick={onHelp}>
          <Keyboard size={15} strokeWidth={1.6} />
          <span className="name">{t('app.allShortcuts')}</span>
          <kbd>{keys('help.toggle')}</kbd>
        </button>
        <button className="nav-item" onClick={() => void reportBug()} title={t('app.reportBugHint')}>
          <Bug size={15} strokeWidth={1.6} />
          <span className="name">{t('app.reportBug')}</span>
          <kbd>{keys('app.reportBug')}</kbd>
        </button>
        <button className="nav-item" onClick={() => setLocale(nextLocale.id)} title={t('app.language')}>
          <Languages size={15} strokeWidth={1.6} />
          <span className="name">{LOCALES.find((l) => l.id === locale)?.label}</span>
          <kbd>{keys('lang.toggle')}</kbd>
        </button>
      </div>
    </aside>
  )
}
