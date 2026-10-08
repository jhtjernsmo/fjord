// Keymap engine: leader key, multi-key sequences, scopes,
// and user overrides from ~/.config/fjord/keymap.json. The mouse always works
// too — the keyboard is an option, never a requirement.

export type Scope = 'global' | 'board'

export interface ActionDef {
  id: string
  /** i18n group key suffix, e.g. 'board' -> 'group.board' */
  group: 'general' | 'go' | 'project' | 'search' | 'view' | 'task' | 'board'
  scope: Scope
  keys: string
}

/** Default bindings. `<leader>` expands to the configured leader key. */
export const DEFAULT_ACTIONS: ActionDef[] = [
  { id: 'palette.open', group: 'general', scope: 'global', keys: 'ctrl+k' },
  { id: 'help.toggle', group: 'general', scope: 'global', keys: '?' },
  { id: 'panel.close', group: 'general', scope: 'global', keys: 'esc' },
  { id: 'go.home', group: 'go', scope: 'global', keys: '<leader>h' },
  { id: 'go.notes', group: 'go', scope: 'global', keys: '<leader>N' },
  { id: 'task.addSubtask', group: 'board', scope: 'global', keys: 'A' },
  { id: 'project.pick', group: 'project', scope: 'global', keys: '<leader>p' },
  { id: 'project.new', group: 'project', scope: 'global', keys: '<leader>n' },
  { id: 'project.archive', group: 'project', scope: 'global', keys: '<leader>P a' },
  { id: 'search.open', group: 'search', scope: 'global', keys: '<leader>f' },
  { id: 'view.board', group: 'view', scope: 'global', keys: '<leader>b' },
  { id: 'view.notes', group: 'view', scope: 'global', keys: '<leader>o' },
  { id: 'view.files', group: 'view', scope: 'global', keys: '<leader>F' },
  { id: 'view.activity', group: 'view', scope: 'global', keys: '<leader>a' },
  { id: 'task.new', group: 'task', scope: 'global', keys: '<leader>t' },
  { id: 'lang.toggle', group: 'general', scope: 'global', keys: '<leader>L' },
  { id: 'settings.open', group: 'general', scope: 'global', keys: '<leader>,' },
  { id: 'theme.cycle', group: 'general', scope: 'global', keys: '<leader>T' },
  { id: 'view.archive', group: 'view', scope: 'global', keys: '<leader>A' },
  { id: 'view.git', group: 'view', scope: 'global', keys: '<leader>g' },
  { id: 'board.left', group: 'board', scope: 'board', keys: 'h' },
  { id: 'board.right', group: 'board', scope: 'board', keys: 'l' },
  { id: 'board.down', group: 'board', scope: 'board', keys: 'j' },
  { id: 'board.up', group: 'board', scope: 'board', keys: 'k' },
  { id: 'board.first', group: 'board', scope: 'board', keys: 'g g' },
  { id: 'board.last', group: 'board', scope: 'board', keys: 'G' },
  { id: 'task.moveLeft', group: 'board', scope: 'board', keys: 'H' },
  { id: 'task.moveRight', group: 'board', scope: 'board', keys: 'L' },
  { id: 'task.moveDown', group: 'board', scope: 'board', keys: 'J' },
  { id: 'task.moveUp', group: 'board', scope: 'board', keys: 'K' },
  { id: 'task.open', group: 'board', scope: 'board', keys: 'enter' },
  { id: 'task.newHere', group: 'board', scope: 'board', keys: 'o' },
  { id: 'task.toggleDone', group: 'board', scope: 'board', keys: 'x' },
  { id: 'task.archive', group: 'board', scope: 'board', keys: 'd d' },
  { id: 'task.priority', group: 'board', scope: 'board', keys: 'p' },
  { id: 'board.filter', group: 'board', scope: 'board', keys: '/' },
  { id: 'task.branch', group: 'board', scope: 'board', keys: 'B' },
  { id: 'task.delete', group: 'board', scope: 'board', keys: 'delete' },
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

/** On macOS, Cmd plays the role of Ctrl so "ctrl+k" also means ⌘K. */
export const IS_MAC = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform)

/** KeyboardEvent -> token like "j", "J", "ctrl+k", "space", "esc". */
export function eventToken(e: KeyboardEvent, mac: boolean = IS_MAC): string | null {
  if (['Shift', 'Control', 'Alt', 'Meta'].includes(e.key)) return null
  const named = NAMED_KEYS[e.key]
  const key = named ?? (e.key.length === 1 ? e.key : e.key.toLowerCase())
  const mods: string[] = []
  if (e.ctrlKey || (mac && e.metaKey)) mods.push('ctrl')
  if (e.altKey) mods.push('alt')
  if (e.metaKey && !mac) mods.push('meta')
  // Shift is already reflected in printable keys ("J"); only spell it for named keys.
  if (e.shiftKey && key.length > 1) mods.push('shift')
  // ctrl+K and ctrl+k should be the same binding.
  const normalized = mods.length > 0 && key.length === 1 ? key.toLowerCase() : key
  return [...mods, normalized].join('+')
}

export function displayKeys(keys: string, leader: string, mac: boolean = IS_MAC): string {
  const pretty: Record<string, string> = { space: '␣', esc: 'Esc', enter: '⏎' }
  return parseSequence(keys, leader)
    .map((k) => pretty[k] ?? (mac ? k.replace(/^ctrl\+/, '⌘') : k))
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

