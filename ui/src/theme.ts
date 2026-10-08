// Themes: built-in and user-made, applied as CSS variables on <html>.
// Pure functions only; React state lives in themes.tsx.

export interface ThemeColors {
  bg: string
  bg2: string
  panel: string
  panel2: string
  line: string
  line2: string
  text: string
  muted: string
  dim: string
  accent: string
  ok: string
  warn: string
  danger: string
}

export type Density = 'compact' | 'cozy' | 'comfortable'

export interface ThemeDef {
  id: string
  name: string
  dark: boolean
  colors: ThemeColors
  /** 0.85–1.25, scales the whole UI. */
  fontScale: number
  /** Corner radius in px for cards and panels (0–18). */
  radius: number
  density: Density
  builtin?: boolean
}

export interface ThemePrefs {
  /** 'system' picks `light` or `dark` from the OS; 'fixed' always uses `fixed`. */
  mode: 'system' | 'fixed'
  fixed: string
  light: string
  dark: string
  /** Let the active project's color replace the theme's accent. */
  projectAccent: boolean
}

const base = { fontScale: 1, radius: 12, density: 'cozy' as Density, builtin: true }

export const BUILTIN_THEMES: ThemeDef[] = [
  {
    ...base,
    id: 'fjord-dark',
    name: 'Fjord Dark',
    dark: true,
    colors: { bg: '#0b0d12', bg2: '#10131a', panel: '#151922', panel2: '#1b202b', line: '#1f232b', line2: '#2b3039', text: '#e8eaf0', muted: '#9097a6', dim: '#5e6575', accent: '#7c9cff', ok: '#3fb950', warn: '#f5a524', danger: '#ff6b6b' },
  },
  {
    ...base,
    id: 'fjord-light',
    name: 'Fjord Light',
    dark: false,
    colors: { bg: '#f5f6f8', bg2: '#eef0f4', panel: '#ffffff', panel2: '#f7f8fa', line: '#e3e5e9', line2: '#d0d3d9', text: '#161a22', muted: '#5b6270', dim: '#8b919d', accent: '#4c6ef5', ok: '#1f8a3b', warn: '#b7791f', danger: '#d64545' },
  },
  {
    ...base,
    id: 'nord',
    name: 'Nord',
    dark: true,
    colors: { bg: '#2e3440', bg2: '#323946', panel: '#3b4252', panel2: '#434c5e', line: '#434c5e', line2: '#4c566a', text: '#eceff4', muted: '#c0c8d6', dim: '#8892a6', accent: '#88c0d0', ok: '#a3be8c', warn: '#ebcb8b', danger: '#bf616a' },
  },
  {
    ...base,
    id: 'solarized-light',
    name: 'Solarized Light',
    dark: false,
    colors: { bg: '#fdf6e3', bg2: '#f5efdc', panel: '#fffbef', panel2: '#eee8d5', line: '#e6dfc8', line2: '#d6cfb6', text: '#073642', muted: '#586e75', dim: '#93a1a1', accent: '#268bd2', ok: '#859900', warn: '#b58900', danger: '#dc322f' },
  },
  {
    ...base,
    id: 'high-contrast',
    name: 'High Contrast',
    dark: true,
    radius: 6,
    colors: { bg: '#000000', bg2: '#0a0a0a', panel: '#111111', panel2: '#1c1c1c', line: '#5a5a5a', line2: '#8a8a8a', text: '#ffffff', muted: '#e0e0e0', dim: '#bdbdbd', accent: '#ffd400', ok: '#4cff7a', warn: '#ffb020', danger: '#ff5c5c' },
  },
]

export const DEFAULT_PREFS: ThemePrefs = { mode: 'system', fixed: 'fjord-dark', light: 'fjord-light', dark: 'fjord-dark', projectAccent: true }

export const COLOR_KEYS: (keyof ThemeColors)[] = ['bg', 'bg2', 'panel', 'panel2', 'line', 'line2', 'text', 'muted', 'dim', 'accent', 'ok', 'warn', 'danger']

const CSS_VAR: Record<keyof ThemeColors, string> = {
  bg: '--bg',
  bg2: '--bg-2',
  panel: '--panel',
  panel2: '--panel-2',
  line: '--line',
  line2: '--line-2',
  text: '--text',
  muted: '--muted',
  dim: '--dim',
  accent: '--accent',
  ok: '--ok',
  warn: '--warn',
  danger: '--danger',
}

/** Writes a theme onto <html>. The accent may then be overridden by the project color. */
export function applyThemeDef(theme: ThemeDef, root: HTMLElement = document.documentElement): void {
  for (const key of COLOR_KEYS) root.style.setProperty(CSS_VAR[key], theme.colors[key])
  root.style.setProperty('--radius', `${theme.radius}px`)
  root.style.setProperty('--radius-sm', `${Math.max(0, Math.round(theme.radius * 0.66))}px`)
  root.style.setProperty('--shadow', theme.dark ? '0 20px 50px -20px rgba(0, 0, 0, 0.7)' : '0 20px 50px -24px rgba(20, 24, 34, 0.25)')
  root.style.colorScheme = theme.dark ? 'dark' : 'light'
  root.dataset.theme = theme.dark ? 'dark' : 'light'
  root.dataset.density = theme.density
  root.style.setProperty('zoom', String(clamp(theme.fontScale, 0.85, 1.25)))
}

export function clamp(n: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, n))
}

/** Which theme id to use right now. */
export function activeThemeId(prefs: ThemePrefs, systemDark: boolean): string {
  if (prefs.mode === 'fixed') return prefs.fixed
  return systemDark ? prefs.dark : prefs.light
}

export function findTheme(id: string, custom: ThemeDef[]): ThemeDef {
  return custom.find((t) => t.id === id) ?? BUILTIN_THEMES.find((t) => t.id === id) ?? BUILTIN_THEMES[0]
}

/** A copy of `from` the user can edit. */
export function duplicateTheme(from: ThemeDef, name: string, existing: ThemeDef[]): ThemeDef {
  let n = 1
  while (existing.some((t) => t.id === `custom-${n}`)) n++
  return { ...structuredClone(from), id: `custom-${n}`, name, builtin: false }
}

// ---------- contrast (WCAG 2) ----------

function luminance(hex: string): number {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim())
  if (!m) return 0
  const n = parseInt(m[1], 16)
  const channel = (c: number) => {
    const s = c / 255
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * channel((n >> 16) & 255) + 0.7152 * channel((n >> 8) & 255) + 0.0722 * channel(n & 255)
}

export function contrastRatio(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x)
  return (hi + 0.05) / (lo + 0.05)
}

/** Pairs that are hard to read: body text needs 4.5:1, secondary text 3:1. */
export function contrastWarnings(c: ThemeColors): { pair: string; ratio: number }[] {
  const checks: [string, string, string, number][] = [
    ['text / background', c.text, c.bg, 4.5],
    ['text / panel', c.text, c.panel, 4.5],
    ['muted / panel', c.muted, c.panel, 3],
    ['accent / panel', c.accent, c.panel, 3],
  ]
  return checks
    .map(([pair, fg, bg, min]) => ({ pair, ratio: contrastRatio(fg, bg), min }))
    .filter((r) => r.ratio < r.min)
    .map(({ pair, ratio }) => ({ pair, ratio: Math.round(ratio * 10) / 10 }))
}

// ---------- import / export ----------

const HEX = /^#[0-9a-f]{6}$/i

/** Validates a theme from a file; returns null if it isn't one. */
export function parseTheme(json: string): Omit<ThemeDef, 'id' | 'builtin'> | null {
  try {
    const raw: unknown = JSON.parse(json)
    if (!raw || typeof raw !== 'object') return null
    const t = raw as Partial<ThemeDef>
    if (typeof t.name !== 'string' || !t.name.trim() || !t.colors) return null
    const colors = {} as ThemeColors
    for (const key of COLOR_KEYS) {
      const v = (t.colors as Partial<ThemeColors>)[key]
      if (typeof v !== 'string' || !HEX.test(v)) return null
      colors[key] = v
    }
    return {
      name: t.name.trim().slice(0, 40),
      dark: !!t.dark,
      colors,
      fontScale: clamp(Number(t.fontScale) || 1, 0.85, 1.25),
      radius: clamp(Number(t.radius ?? 12), 0, 18),
      density: t.density === 'compact' || t.density === 'comfortable' ? t.density : 'cozy',
    }
  } catch {
    return null
  }
}

export function exportTheme(theme: ThemeDef): string {
  const { name, dark, colors, fontScale, radius, density } = theme
  return JSON.stringify({ fjordTheme: 1, name, dark, colors, fontScale, radius, density }, null, 2)
}

// ---------- storage ----------

const THEMES_KEY = 'fjord.themes'
const PREFS_KEY = 'fjord.themePrefs'
const LEGACY_KEY = 'fjord.theme'

export function loadCustomThemes(): ThemeDef[] {
  try {
    const raw: unknown = JSON.parse(localStorage.getItem(THEMES_KEY) ?? '[]')
    if (!Array.isArray(raw)) return []
    return raw.flatMap((t) => {
      const parsed = parseTheme(JSON.stringify(t))
      return parsed && typeof (t as ThemeDef).id === 'string' ? [{ ...parsed, id: (t as ThemeDef).id, builtin: false }] : []
    })
  } catch {
    return []
  }
}

export function saveCustomThemes(themes: ThemeDef[]): void {
  try {
    localStorage.setItem(THEMES_KEY, JSON.stringify(themes))
  } catch {
    /* non-fatal */
  }
}

export function loadPrefs(): ThemePrefs {
  try {
    const stored = localStorage.getItem(PREFS_KEY)
    if (stored) return { ...DEFAULT_PREFS, ...(JSON.parse(stored) as Partial<ThemePrefs>) }
    // Migrate the old System / Dark / Light setting.
    const legacy = localStorage.getItem(LEGACY_KEY)
    if (legacy === 'dark') return { ...DEFAULT_PREFS, mode: 'fixed', fixed: 'fjord-dark' }
    if (legacy === 'light') return { ...DEFAULT_PREFS, mode: 'fixed', fixed: 'fjord-light' }
  } catch {
    /* fall through */
  }
  return DEFAULT_PREFS
}

export function savePrefs(prefs: ThemePrefs): void {
  try {
    localStorage.setItem(PREFS_KEY, JSON.stringify(prefs))
  } catch {
    /* non-fatal */
  }
}
