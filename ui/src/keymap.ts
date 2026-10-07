// LazyVim-inspired keymap engine: leader key, multi-key sequences, scopes,
// and user overrides from ~/.config/fjord/keymap.json. The mouse always works
// too — the keyboard is an option, never a requirement.

export type Scope = 'global' | 'board'

export interface ActionDef {
  id: string
  label: string
  group: string
  scope: Scope
  keys: string
}

/** Default bindings. `<leader>` expands to the configured leader key. */
export const DEFAULT_ACTIONS: ActionDef[] = [
  { id: 'palette.open', label: 'Kommandopalett', group: 'Generelt', scope: 'global', keys: 'ctrl+k' },
  { id: 'help.toggle', label: 'Vis hurtigtaster', group: 'Generelt', scope: 'global', keys: '?' },
  { id: 'panel.close', label: 'Lukk', group: 'Generelt', scope: 'global', keys: 'esc' },
  { id: 'go.home', label: 'Hjem / oversikt', group: 'Gå til', scope: 'global', keys: '<leader>h' },
  { id: 'project.pick', label: 'Bytt prosjekt', group: 'Prosjekt', scope: 'global', keys: '<leader>p' },
  { id: 'project.new', label: 'Nytt prosjekt', group: 'Prosjekt', scope: 'global', keys: '<leader>n' },
  { id: 'project.archive', label: 'Arkiver prosjekt', group: 'Prosjekt', scope: 'global', keys: '<leader>P a' },
  { id: 'search.open', label: 'Søk overalt', group: 'Søk', scope: 'global', keys: '<leader>f' },
  { id: 'view.board', label: 'Tavle', group: 'Visning', scope: 'global', keys: '<leader>b' },
  { id: 'view.notes', label: 'Notater', group: 'Visning', scope: 'global', keys: '<leader>o' },
  { id: 'view.files', label: 'Filer', group: 'Visning', scope: 'global', keys: '<leader>F' },
  { id: 'view.activity', label: 'Aktivitet', group: 'Visning', scope: 'global', keys: '<leader>a' },
  { id: 'task.new', label: 'Ny oppgave', group: 'Oppgave', scope: 'global', keys: '<leader>t' },
  { id: 'board.left', label: 'Kolonne til venstre', group: 'Tavle', scope: 'board', keys: 'h' },
  { id: 'board.right', label: 'Kolonne til høyre', group: 'Tavle', scope: 'board', keys: 'l' },
  { id: 'board.down', label: 'Neste oppgave', group: 'Tavle', scope: 'board', keys: 'j' },
  { id: 'board.up', label: 'Forrige oppgave', group: 'Tavle', scope: 'board', keys: 'k' },
  { id: 'board.first', label: 'Første i kolonnen', group: 'Tavle', scope: 'board', keys: 'g g' },
  { id: 'board.last', label: 'Siste i kolonnen', group: 'Tavle', scope: 'board', keys: 'G' },
  { id: 'task.moveLeft', label: 'Flytt oppgave venstre', group: 'Tavle', scope: 'board', keys: 'H' },
  { id: 'task.moveRight', label: 'Flytt oppgave høyre', group: 'Tavle', scope: 'board', keys: 'L' },
  { id: 'task.moveDown', label: 'Flytt oppgave ned', group: 'Tavle', scope: 'board', keys: 'J' },
  { id: 'task.moveUp', label: 'Flytt oppgave opp', group: 'Tavle', scope: 'board', keys: 'K' },
  { id: 'task.open', label: 'Åpne oppgave', group: 'Tavle', scope: 'board', keys: 'enter' },
  { id: 'task.newHere', label: 'Ny oppgave i kolonnen', group: 'Tavle', scope: 'board', keys: 'o' },
  { id: 'task.toggleDone', label: 'Ferdig / ikke ferdig', group: 'Tavle', scope: 'board', keys: 'x' },
  { id: 'task.archive', label: 'Arkiver oppgave', group: 'Tavle', scope: 'board', keys: 'd d' },
  { id: 'task.priority', label: 'Bytt prioritet', group: 'Tavle', scope: 'board', keys: 'p' },
]

export interface KeymapConfig {
  leader: string
  /** action id -> key sequence */
  bindings: Record<string, string>
}

export const DEFAULT_LEADER = 'space'

/** Merges a user config (unknown JSON) over the defaults, ignoring junk. */
export function buildKeymap(user: unknown): KeymapConfig {
  const bindings: Record<string, string> = Object.fromEntries(DEFAULT_ACTIONS.map((a) => [a.id, a.keys]))
  let leader = DEFAULT_LEADER
  if (user && typeof user === 'object') {
    const u = user as { leader?: unknown; bindings?: unknown }
    if (typeof u.leader === 'string' && u.leader.trim()) leader = u.leader.trim().toLowerCase()
    if (u.bindings && typeof u.bindings === 'object') {
      for (const [id, keys] of Object.entries(u.bindings as Record<string, unknown>)) {
        if (id in bindings && typeof keys === 'string' && keys.trim()) bindings[id] = keys.trim()
      }
    }
  }
  return { leader, bindings }
}

/** "<leader>P a" -> ["space", "P", "a"]; single chars keep their case. */
export function parseSequence(keys: string, leader: string): string[] {
  return keys
    .replace(/<leader>/g, ` ${leader} `)
    .split(/\s+/)
    .filter(Boolean)
    .map((k) => (k.length === 1 ? k : k.toLowerCase()))
}

const NAMED_KEYS: Record<string, string> = {
  ' ': 'space',
  Escape: 'esc',
  Enter: 'enter',
  Tab: 'tab',
  Backspace: 'backspace',
}

/** KeyboardEvent -> token like "j", "J", "ctrl+k", "space", "esc". */
export function eventToken(e: KeyboardEvent): string | null {
  if (['Shift', 'Control', 'Alt', 'Meta'].includes(e.key)) return null
  const named = NAMED_KEYS[e.key]
  const key = named ?? (e.key.length === 1 ? e.key : e.key.toLowerCase())
  const mods: string[] = []
  if (e.ctrlKey) mods.push('ctrl')
  if (e.altKey) mods.push('alt')
  if (e.metaKey) mods.push('meta')
  // Shift is already reflected in printable keys ("J"); only spell it for named keys.
  if (e.shiftKey && key.length > 1) mods.push('shift')
  // ctrl+K and ctrl+k should be the same binding.
  const normalized = mods.length > 0 && key.length === 1 ? key.toLowerCase() : key
  return [...mods, normalized].join('+')
}

export function displayKeys(keys: string, leader: string): string {
  const pretty: Record<string, string> = { space: '␣', esc: 'Esc', enter: '⏎' }
  return parseSequence(keys, leader)
    .map((k) => pretty[k] ?? k)
    .join(' ')
}

export interface Continuation {
  key: string
  actionId: string
  /** true when more keys follow (a group, like "P" in "<leader>P a"). */
  isGroup: boolean
}

export interface Match {
  kind: 'none' | 'prefix' | 'exact'
  actionId?: string
  next?: Continuation[]
}

/** Finds what a pending key sequence means among the active scopes. */
export function match(pending: string[], config: KeymapConfig, scopes: Scope[]): Match {
  let exact: string | undefined
  const next: Continuation[] = []
  for (const action of DEFAULT_ACTIONS) {
    if (!scopes.includes(action.scope)) continue
    const seq = parseSequence(config.bindings[action.id], config.leader)
    if (seq.length < pending.length || !pending.every((k, i) => seq[i] === k)) continue
    if (seq.length === pending.length) exact = action.id
    else next.push({ key: seq[pending.length], actionId: action.id, isGroup: seq.length > pending.length + 1 })
  }
  if (next.length > 0) return { kind: 'prefix', actionId: exact, next }
  if (exact) return { kind: 'exact', actionId: exact }
  return { kind: 'none' }
}

export function isTypingTarget(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null
  if (!el) return false
  return el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT' || el.isContentEditable
}

export function actionLabel(id: string): string {
  return DEFAULT_ACTIONS.find((a) => a.id === id)?.label ?? id
}
