import { editorLabel, openInEditor } from './editor'
import { useProjectSort } from './projectSort'
import { useCallback, useEffect, useState } from 'react'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { Ellipsis, Paperclip } from 'lucide-react'
import { useContextMenus } from './components/actions'
import { api, errorMessage } from './api'
import type { SearchHit, Project } from './api'
import { useActions, useApp, useLive } from './data'
import { NavContext } from './nav'
import type { Nav } from './nav'
import { Notespace } from './components/Notespace'
import { UpdateBanner } from './components/Updater'
import { TitleBar } from './components/TitleBar'
import { reportBug } from './reportBug'
import { refreshPullRequests } from './components/GitView'
import { AzureImportRunner } from './components/AzureImport'
import { MentionsRunner } from './components/Mentions'
import { LOCALES } from './i18n'
import type { MessageKey } from './i18n'
import { ProjectGlyph } from './components/Icons'
import { Board } from './components/Board'
import { CommandPalette } from './components/CommandPalette'
import type { PaletteMode } from './components/CommandPalette'
import { NewProjectDialog } from './components/Dialogs'
import { rememberParent } from './repoName'
import { SettingsWindow } from './components/Settings'
import { useThemes } from './themes'
import { Home } from './components/Home'
import { ActivityView, ArchiveView, FilesView, NotesView } from './components/ProjectViews'
import { GitView } from './components/GitView'
import { Sidebar } from './components/Sidebar'
import { TaskPanel } from './components/TaskPanel'
import { HelpSheet, WhichKey } from './components/WhichKey'

type Tab = 'board' | 'notes' | 'files' | 'git' | 'activity' | 'archive'
type View = { kind: 'home' } | { kind: 'notes' } | { kind: 'project'; id: number; tab: Tab }

const TABS: Tab[] = ['board', 'notes', 'files', 'git', 'activity', 'archive']

const GLOBAL_ACTIONS = [
  'palette.open', 'help.toggle', 'panel.close', 'go.home', 'go.notes', 'project.pick', 'project.new', 'project.archive',
  'search.open', 'view.board', 'view.notes', 'view.files', 'view.activity', 'task.new', 'lang.toggle', 'settings.open', 'theme.cycle', 'app.reportBug', 'git.sync', 'git.createRepo', 'azure.import', 'view.archive', 'view.git', 'editor.open',
]

export default function App() {
  const { run, toast, t, locale, setLocale } = useApp()
  const { projectMenu } = useContextMenus()
  const [view, setView] = useState<View>({ kind: 'home' })
  const [taskId, setTaskId] = useState<number | null>(null)
  const [noteId, setNoteId] = useState<number | null>(null)
  const [editingProject, setEditingProject] = useState<Project | null>(null)
  const [palette, setPalette] = useState<PaletteMode | null>(null)
  const [newProject, setNewProject] = useState(false)
  const [help, setHelp] = useState(false)
  const [settings, setSettings] = useState(false)
  const themes = useThemes()
  const [dropping, setDropping] = useState(false)
  const [quickAddSignal, setQuickAddSignal] = useState(0)
  const [openCreateRepo, setOpenCreateRepo] = useState(false)
  const closeCreateRepo = useCallback(() => setOpenCreateRepo(false), [])

  const [actor] = useLive(() => api.actor(), [])
  const [projects] = useLive(() => api.listProjects(false), [])
  const projectId = view.kind === 'project' ? view.id : null
  const { sorted: sortedProjects, sort: projectSort, setSort: setProjectSort } = useProjectSort(projects ?? [], projectId)
  const [board] = useLive(() => (projectId ? api.getBoard(projectId) : Promise.resolve(null)), [projectId])
  const activeBoard = board && board.project.id === projectId ? board : null
  const windowTitle = view.kind === 'notes' ? t('nav.notes') : activeBoard ? activeBoard.project.name : 'Fjord'

  const openProject = useCallback((id: number, tab: Tab = 'board') => {
    setView({ kind: 'project', id, tab })
    setTaskId(null)
  }, [])
  const setTab = (tab: Tab) => projectId && setView({ kind: 'project', id: projectId, tab })

  const overlayOpen = palette !== null || newProject || help || settings

  // The accent follows the active project's colour, unless turned off in Appearance.
  const projectColor = themes.prefs.projectAccent ? activeBoard?.project.color : null
  useEffect(() => {
    document.documentElement.style.setProperty('--accent', projectColor ?? themes.current.colors.accent)
  }, [projectColor, themes.current])

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
      'go.notes': () => {
        setView({ kind: 'notes' })
        setTaskId(null)
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
      'theme.cycle': () => themes.cycle(),
      'app.reportBug': () => void reportBug(),
      'editor.open': () => {
        if (!projectId) return toast(t('toast.pickProject'), 'info')
        openInEditor(projectId, taskId)
          .then(() => toast(t('editor.opened', { editor: editorLabel() }), 'success'))
          .catch((err) => toast(errorMessage(err), 'error'))
      },
      'git.sync': () => {
        if (!projectId) return
        refreshPullRequests(projectId)
          .then((r) => toast(t('git.synced', { n: r.pull_requests.length }), 'success'))
          .catch((err) => toast(errorMessage(err), 'error'))
      },
      'git.createRepo': () => {
        if (!projectId) return toast(t('toast.pickProject'), 'info')
        setTaskId(null)
        setView({ kind: 'project', id: projectId, tab: 'git' })
        api
          .getProjectRepo(projectId)
          .then((repo) => (repo ? toast(t('newRepo.alreadyLinked'), 'info') : setOpenCreateRepo(true)))
          .catch((err) => toast(errorMessage(err), 'error'))
      },
      'azure.import': () =>
        api
          .runAzureImport()
          .then((r) => toast(t('import.report', { created: r.created.length, updated: r.updated, closed: r.closed, unmapped: r.unmapped }), 'success'))
          .catch((err) => toast(errorMessage(err), 'error')),
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

  const nav: Nav = {
    openProject: (id) => openProject(id),
    openTask: (projectId, id) => {
      setView({ kind: 'project', id: projectId, tab: 'board' })
      setTaskId(id)
    },
    openNote: (id, projectId) => {
      setNoteId(id)
      setTaskId(null)
      setView(projectId === null ? { kind: 'notes' } : { kind: 'project', id: projectId, tab: 'notes' })
    },
    editProject: (project) => setEditingProject(project),
    createNote: async (title) => {
      const note = await run(api.createNote(null, title, ''))
      if (note) nav.openNote(note.id, null)
    },
  }

  const openHit = (hit: SearchHit) => {
    if (hit.kind === 'note') nav.openNote(hit.ref_id, hit.project_id)
    else if (hit.project_id === null) return
    else if (hit.kind === 'task') nav.openTask(hit.project_id, hit.ref_id)
    else openProject(hit.project_id, 'files')
  }

  return (
    <NavContext.Provider value={nav}>
    <div className="app">
      <TitleBar title={windowTitle} />
      <AzureImportRunner />
      <MentionsRunner />
      <Sidebar
        projects={sortedProjects}
        sort={projectSort}
        onSort={setProjectSort}
        activeId={projectId}
        onHome={() => runAction('go.home')}
        onNotes={() => runAction('go.notes')}
        notesActive={view.kind === 'notes'}
        onSelect={(id) => openProject(id)}
        onNewProject={() => setNewProject(true)}
        onPalette={() => setPalette('commands')}
        onHelp={() => setHelp(true)}
        onSettings={() => setSettings(true)}
      />

      <main className="main">
        <UpdateBanner />
        {view.kind === 'notes' && (
          <>
            <header className="header">
              <h1>{t('nav.notes')}</h1>
            </header>
            <div className="page notes-page">
              <Notespace projectId={null} projects={projects ?? []} selectedId={noteId} onSelect={setNoteId} />
            </div>
          </>
        )}

        {view.kind === 'home' && (
          <Home actor={actor ?? ''} projects={sortedProjects} onOpen={openProject} onNewProject={() => setNewProject(true)} keysEnabled={!overlayOpen} />
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
            {view.tab === 'notes' && (
              <NotesView projectId={activeBoard.project.id} projects={projects ?? []} selectedId={noteId} onSelect={setNoteId} />
            )}
            {view.tab === 'files' && <FilesView projectId={activeBoard.project.id} />}
            {view.tab === 'activity' && <ActivityView projectId={activeBoard.project.id} />}
            {view.tab === 'archive' && <ArchiveView projectId={activeBoard.project.id} />}
            {view.tab === 'git' && <GitView projectId={activeBoard.project.id} onOpenTask={setTaskId} openCreateRepo={openCreateRepo} onCreateRepoDone={closeCreateRepo} />}
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
          projects={sortedProjects}
          onClose={() => setPalette(null)}
          onAction={runAction}
          onProject={(id) => openProject(id)}
          onHit={openHit}
        />
      )}
      {newProject && (
        <NewProjectDialog
          onCancel={() => setNewProject(false)}
          onCreate={async (input, repo) => {
            const p = await run(api.createProject(input), repo ? undefined : t('toast.created', { name: input.name }))
            if (p && repo) {
              rememberParent(repo.folder)
              const linked = await run(api.createGithubRepo(p.id, repo.repo, repo.folder))
              if (linked) toast(t('newRepo.created', { name: input.name, repo: `${repo.repo.owner}/${repo.repo.name}` }), 'success')
            }
            setNewProject(false)
            if (p) openProject(p.id)
          }}
        />
      )}
      {editingProject && (
        <NewProjectDialog
          project={editingProject}
          onCancel={() => setEditingProject(null)}
          onCreate={async (input) => {
            const p = await run(
              api.updateProject(editingProject.id, { name: input.name, description: input.description, icon: input.icon, color: input.color }),
              t('toast.saved'),
            )
            if (p) setEditingProject(null)
          }}
        />
      )}
      {help && <HelpSheet onClose={() => setHelp(false)} />}
      {settings && <SettingsWindow onClose={() => setSettings(false)} />}
      <WhichKey />
    </div>
    </NavContext.Provider>
  )
}
