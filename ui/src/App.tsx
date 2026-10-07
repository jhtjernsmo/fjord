import { useCallback, useEffect, useState } from 'react'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { Ellipsis, Paperclip } from 'lucide-react'
import { useContextMenus } from './components/actions'
import { api } from './api'
import type { SearchHit } from './api'
import { useActions, useApp, useLive } from './data'
import { LOCALES } from './i18n'
import type { MessageKey } from './i18n'
import { ProjectGlyph } from './components/Icons'
import { Board } from './components/Board'
import { CommandPalette } from './components/CommandPalette'
import type { PaletteMode } from './components/CommandPalette'
import { applyTheme, loadTheme, NewProjectDialog, SettingsDialog } from './components/Dialogs'
import type { Theme } from './components/Dialogs'
import { Home } from './components/Home'
import { ActivityView, ArchiveView, FilesView, NotesView } from './components/ProjectViews'
import { GitView } from './components/GitView'
import { Sidebar } from './components/Sidebar'
import { TaskPanel } from './components/TaskPanel'
import { HelpSheet, WhichKey } from './components/WhichKey'

type Tab = 'board' | 'notes' | 'files' | 'git' | 'activity' | 'archive'
type View = { kind: 'home' } | { kind: 'project'; id: number; tab: Tab }

const TABS: Tab[] = ['board', 'notes', 'files', 'git', 'activity', 'archive']

const GLOBAL_ACTIONS = [
  'palette.open', 'help.toggle', 'panel.close', 'go.home', 'project.pick', 'project.new', 'project.archive',
  'search.open', 'view.board', 'view.notes', 'view.files', 'view.activity', 'task.new', 'lang.toggle', 'settings.open', 'view.archive', 'view.git',
]

export default function App() {
  const { run, toast, t, locale, setLocale } = useApp()
  const { projectMenu } = useContextMenus()
  const [view, setView] = useState<View>({ kind: 'home' })
  const [taskId, setTaskId] = useState<number | null>(null)
  const [palette, setPalette] = useState<PaletteMode | null>(null)
  const [newProject, setNewProject] = useState(false)
  const [help, setHelp] = useState(false)
  const [settings, setSettings] = useState(false)
  const [theme, setTheme] = useState<Theme>(loadTheme)
  useEffect(() => applyTheme(theme), [theme])
  const [dropping, setDropping] = useState(false)
  const [quickAddSignal, setQuickAddSignal] = useState(0)

  const [actor] = useLive(() => api.actor(), [])
  const [projects] = useLive(() => api.listProjects(false), [])
  const projectId = view.kind === 'project' ? view.id : null
  const [board] = useLive(() => (projectId ? api.getBoard(projectId) : Promise.resolve(null)), [projectId])
  const activeBoard = board && board.project.id === projectId ? board : null

  const openProject = useCallback((id: number, tab: Tab = 'board') => {
    setView({ kind: 'project', id, tab })
    setTaskId(null)
  }, [])
  const setTab = (tab: Tab) => projectId && setView({ kind: 'project', id: projectId, tab })

  const overlayOpen = palette !== null || newProject || help || settings

  // Accent colour follows the active project.
  useEffect(() => {
    document.documentElement.style.setProperty('--accent', activeBoard?.project.color ?? '#7c9cff')
  }, [activeBoard?.project.color])

  // Dropped files go to the open task, else to the current project.
  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent(async (event) => {
      const payload = event.payload
      if (payload.type === 'over' || payload.type === 'enter') setDropping(projectId !== null)
      else if (payload.type === 'leave') setDropping(false)
      else if (payload.type === 'drop') {
        setDropping(false)
        if (!projectId) {
          toast(t('drop.needProject'), 'info')
          return
        }
        const results = await run(api.attachFiles(projectId, taskId, payload.paths))
        const ok = results?.filter((r) => r.attachment).length ?? 0
        results?.filter((r) => r.error).forEach((r) => toast(`${r.path}: ${r.error}`, 'error'))
        if (ok > 0) toast(t('drop.attached', { n: ok }), 'success')
      }
    })
    return () => {
      unlisten.then((fn) => fn())
    }
  }, [projectId, taskId, run, toast, t])

  const runAction = (id: string) => {
    const actions: Record<string, () => void> = {
      'palette.open': () => setPalette('commands'),
      'help.toggle': () => setHelp((h) => !h),
      'panel.close': () => {
        if (settings) setSettings(false)
        else if (help) setHelp(false)
        else if (palette) setPalette(null)
        else if (newProject) setNewProject(false)
        else setTaskId(null)
      },
      'go.home': () => {
        setView({ kind: 'home' })
        setTaskId(null)
      },
      'project.pick': () => setPalette('projects'),
      'project.new': () => setNewProject(true),
      'project.archive': () => {
        if (!activeBoard) return
        run(api.archiveProject(activeBoard.project.id, true), t('toast.archivedProject', { name: activeBoard.project.name }))
        setView({ kind: 'home' })
      },
      'search.open': () => setPalette('search'),
      'view.board': () => setTab('board'),
      'view.notes': () => setTab('notes'),
      'view.files': () => setTab('files'),
      'view.activity': () => setTab('activity'),
      'view.archive': () => setTab('archive'),
      'view.git': () => setTab('git'),
      'settings.open': () => setSettings(true),
      'task.new': () => {
        if (!projectId) {
          toast(t('toast.pickProject'), 'info')
          return
        }
        setTaskId(null)
        setView({ kind: 'project', id: projectId, tab: 'board' })
        setQuickAddSignal((n) => n + 1)
      },
      'lang.toggle': () => setLocale(LOCALES[(LOCALES.findIndex((l) => l.id === locale) + 1) % LOCALES.length].id),
    }
    actions[id]?.()
  }

  useActions(Object.fromEntries(GLOBAL_ACTIONS.map((id) => [id, () => runAction(id)])))

  const openHit = (hit: SearchHit) => {
    if (hit.kind === 'task') {
      setView({ kind: 'project', id: hit.project_id, tab: 'board' })
      setTaskId(hit.ref_id)
    } else {
      openProject(hit.project_id, hit.kind === 'note' ? 'notes' : 'files')
    }
  }

  return (
    <div className="app">
      <Sidebar
        projects={projects ?? []}
        activeId={projectId}
        onHome={() => runAction('go.home')}
        onSelect={(id) => openProject(id)}
        onNewProject={() => setNewProject(true)}
        onPalette={() => setPalette('commands')}
        onHelp={() => setHelp(true)}
        onSettings={() => setSettings(true)}
      />

      <main className="main">
        {view.kind === 'home' && (
          <Home actor={actor ?? ''} projects={projects ?? []} onOpen={openProject} onNewProject={() => setNewProject(true)} />
        )}

        {view.kind === 'project' && activeBoard && (
          <>
            <header className="header">
              <h1>
                <ProjectGlyph glyph={activeBoard.project.icon} color={activeBoard.project.color} size="lg" />
                {activeBoard.project.name}
              </h1>
              <span className="desc">{activeBoard.project.description}</span>
              <nav className="tabs">
                {TABS.map((tab) => (
                  <button key={tab} className={`tab ${view.tab === tab ? 'active' : ''}`} onClick={() => setTab(tab)}>
                    {t(`tab.${tab}` as MessageKey)}
                  </button>
                ))}
              </nav>
              <button
                className="icon-btn header-menu"
                aria-label={t('menu.project')}
                title={t('menu.project')}
                onClick={projectMenu(activeBoard.project, { onOpen: () => undefined, onGone: () => setView({ kind: 'home' }) })}
              >
                <Ellipsis size={16} />
              </button>
            </header>
            {view.tab === 'board' && (
              <Board
                board={activeBoard}
                selectedTaskId={taskId}
                onOpenTask={setTaskId}
                quickAddSignal={quickAddSignal}
                keysEnabled={taskId === null && !overlayOpen}
              />
            )}
            {view.tab === 'notes' && <NotesView projectId={activeBoard.project.id} />}
            {view.tab === 'files' && <FilesView projectId={activeBoard.project.id} />}
            {view.tab === 'activity' && <ActivityView projectId={activeBoard.project.id} />}
            {view.tab === 'archive' && <ArchiveView projectId={activeBoard.project.id} />}
            {view.tab === 'git' && <GitView projectId={activeBoard.project.id} onOpenTask={setTaskId} />}
            {taskId !== null && (
              <TaskPanel key={taskId} taskId={taskId} statuses={activeBoard.statuses} onClose={() => setTaskId(null)} />
            )}
          </>
        )}

        {dropping && (
          <div className="drop-overlay">
            <Paperclip size={22} /> {taskId ? t('drop.task') : t('drop.project')}
          </div>
        )}
      </main>

      {palette && (
        <CommandPalette
          mode={palette}
          projects={projects ?? []}
          onClose={() => setPalette(null)}
          onAction={runAction}
          onProject={(id) => openProject(id)}
          onHit={openHit}
        />
      )}
      {newProject && (
        <NewProjectDialog
          onCancel={() => setNewProject(false)}
          onCreate={async (input) => {
            const p = await run(api.createProject(input), t('toast.created', { name: input.name }))
            setNewProject(false)
            if (p) openProject(p.id)
          }}
        />
      )}
      {help && <HelpSheet onClose={() => setHelp(false)} />}
      {settings && <SettingsDialog theme={theme} onTheme={setTheme} onClose={() => setSettings(false)} />}
      <WhichKey />
    </div>
  )
}
