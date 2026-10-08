// Theme state for the whole app: preferences, custom themes, live preview while editing.
import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'
import type { ReactNode } from 'react'
import { BUILTIN_THEMES, activeThemeId, applyThemeDef, findTheme, loadCustomThemes, loadPrefs, saveCustomThemes, savePrefs } from './theme'
import type { ThemeDef, ThemePrefs } from './theme'

interface ThemeState {
  prefs: ThemePrefs
  setPrefs: (p: ThemePrefs) => void
  custom: ThemeDef[]
  all: ThemeDef[]
  /** The theme on screen (the preview while editing, otherwise the active one). */
  current: ThemeDef
  saveTheme: (t: ThemeDef) => void
  deleteTheme: (id: string) => void
  /** Show a theme without saving it (the editor); null ends the preview. */
  preview: (t: ThemeDef | null) => void
  /** Switch to the next theme (fixed mode). */
  cycle: () => void
}

const ThemeContext = createContext<ThemeState | null>(null)

function useSystemDark(): boolean {
  const query = typeof window !== 'undefined' ? window.matchMedia?.('(prefers-color-scheme: dark)') : undefined
  const [dark, setDark] = useState(query?.matches ?? true)
  useEffect(() => {
    if (!query) return
    const onChange = (e: MediaQueryListEvent) => setDark(e.matches)
    query.addEventListener('change', onChange)
    return () => query.removeEventListener('change', onChange)
  }, [query])
  return dark
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [prefs, setPrefsState] = useState(loadPrefs)
  const [custom, setCustom] = useState(loadCustomThemes)
  const [previewing, setPreviewing] = useState<ThemeDef | null>(null)
  const systemDark = useSystemDark()

  const all = useMemo(() => [...BUILTIN_THEMES, ...custom], [custom])
  const active = findTheme(activeThemeId(prefs, systemDark), custom)
  const current = previewing ?? active

  useEffect(() => applyThemeDef(current), [current])

  const setPrefs = useCallback((p: ThemePrefs) => {
    setPrefsState(p)
    savePrefs(p)
  }, [])

  const saveTheme = useCallback((t: ThemeDef) => {
    setCustom((list) => {
      const next = list.some((x) => x.id === t.id) ? list.map((x) => (x.id === t.id ? t : x)) : [...list, t]
      saveCustomThemes(next)
      return next
    })
  }, [])

  const deleteTheme = useCallback(
    (id: string) => {
      setCustom((list) => {
        const next = list.filter((x) => x.id !== id)
        saveCustomThemes(next)
        return next
      })
      // Anything pointing at the deleted theme falls back to the defaults.
      const fix = (v: string, fallback: string) => (v === id ? fallback : v)
      setPrefs({ ...prefs, fixed: fix(prefs.fixed, 'fjord-dark'), dark: fix(prefs.dark, 'fjord-dark'), light: fix(prefs.light, 'fjord-light') })
    },
    [prefs, setPrefs],
  )

  const cycle = useCallback(() => {
    const i = all.findIndex((t) => t.id === active.id)
    setPrefs({ ...prefs, mode: 'fixed', fixed: all[(i + 1) % all.length].id })
  }, [all, active.id, prefs, setPrefs])

  const value = useMemo(
    () => ({ prefs, setPrefs, custom, all, current, saveTheme, deleteTheme, preview: setPreviewing, cycle }),
    [prefs, setPrefs, custom, all, current, saveTheme, deleteTheme, cycle],
  )
  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
}

export function useThemes(): ThemeState {
  const ctx = useContext(ThemeContext)
  if (!ctx) throw new Error('useThemes needs a ThemeProvider')
  return ctx
}
