// Which editor or IDE "Open in editor" starts. Per device (editors are per machine),
// kept in localStorage like the theme.
import { api } from './api'

export interface EditorPreset {
  id: string
  label: string
  /** Command template; `{path}` is the repository folder (added at the end if missing). */
  command: string
}

const isMac = typeof navigator !== 'undefined' && /Mac/i.test(navigator.userAgent)
const isWindows = typeof navigator !== 'undefined' && /Windows/i.test(navigator.userAgent)

/** On macOS, apps launched from the Dock don't see shell launchers like `code`, so open the app itself. */
const app = (label: string, cli: string, macApp = label) => (isMac ? `open -a "${macApp}" {path}` : `${cli} {path}`)

export const EDITOR_PRESETS: EditorPreset[] = [
  { id: 'vscode', label: 'VS Code', command: app('VS Code', 'code', 'Visual Studio Code') },
  { id: 'cursor', label: 'Cursor', command: app('Cursor', 'cursor') },
  { id: 'vscodium', label: 'VSCodium', command: app('VSCodium', 'codium') },
  { id: 'zed', label: 'Zed', command: app('Zed', 'zed') },
  { id: 'idea', label: 'IntelliJ IDEA', command: app('IntelliJ IDEA', 'idea') },
  { id: 'webstorm', label: 'WebStorm', command: app('WebStorm', 'webstorm') },
  { id: 'rider', label: 'Rider', command: app('Rider', 'rider') },
  { id: 'pycharm', label: 'PyCharm', command: app('PyCharm', 'pycharm') },
  { id: 'sublime', label: 'Sublime Text', command: app('Sublime Text', 'subl') },
  ...(isWindows ? [{ id: 'visualstudio', label: 'Visual Studio', command: 'devenv {path}' }] : []),
]

export interface EditorChoice {
  /** A preset id, or 'custom'. */
  id: string
  custom: string
}

const KEY = 'fjord.editor'
export const DEFAULT_EDITOR: EditorChoice = { id: 'vscode', custom: '' }

export function loadEditor(): EditorChoice {
  try {
    const raw = localStorage.getItem(KEY)
    return raw ? { ...DEFAULT_EDITOR, ...JSON.parse(raw) } : DEFAULT_EDITOR
  } catch {
    return DEFAULT_EDITOR
  }
}

export function saveEditor(choice: EditorChoice) {
  try {
    localStorage.setItem(KEY, JSON.stringify(choice))
  } catch {
    /* storage unavailable: the choice lasts until restart */
  }
}

export function editorCommand(choice: EditorChoice = loadEditor()): string {
  if (choice.id === 'custom') return choice.custom.trim()
  return (EDITOR_PRESETS.find((p) => p.id === choice.id) ?? EDITOR_PRESETS[0]).command
}

export function editorLabel(choice: EditorChoice = loadEditor()): string {
  if (choice.id === 'custom') return choice.custom.trim().split(/\s+/)[0] || 'editor'
  return (EDITOR_PRESETS.find((p) => p.id === choice.id) ?? EDITOR_PRESETS[0]).label
}

/** Opens the project's repo in the chosen editor; with a task, on its branch. */
export const openInEditor = (projectId: number, taskId: number | null = null) =>
  api.openInEditor(projectId, taskId, editorCommand())
