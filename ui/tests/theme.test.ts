// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from 'vitest'
import {
  BUILTIN_THEMES,
  DEFAULT_PREFS,
  activeThemeId,
  applyThemeDef,
  contrastRatio,
  contrastWarnings,
  duplicateTheme,
  exportTheme,
  findTheme,
  loadPrefs,
  parseTheme,
} from '../src/theme'

describe('themes', () => {
  beforeEach(() => localStorage.clear())

  it('built-in themes have readable text', () => {
    for (const theme of BUILTIN_THEMES) expect(contrastWarnings(theme.colors), theme.name).toEqual([])
    expect(contrastRatio('#000000', '#ffffff')).toBeCloseTo(21, 0)
  })

  it('flags unreadable custom colours', () => {
    const bad = { ...BUILTIN_THEMES[0].colors, text: '#20242c' }
    expect(contrastWarnings(bad).map((w) => w.pair)).toContain('text / background')
  })

  it('picks the theme for the OS mode, or the fixed one', () => {
    expect(activeThemeId(DEFAULT_PREFS, true)).toBe('fjord-dark')
    expect(activeThemeId(DEFAULT_PREFS, false)).toBe('fjord-light')
    expect(activeThemeId({ ...DEFAULT_PREFS, mode: 'fixed', fixed: 'nord' }, false)).toBe('nord')
    expect(findTheme('missing', []).id).toBe('fjord-dark')
  })

  it('exports and imports themes, rejecting anything that is not one', () => {
    const copy = duplicateTheme(BUILTIN_THEMES[2], 'My Nord', [])
    expect(copy.id).toBe('custom-1')
    expect(copy.builtin).toBe(false)
    const parsed = parseTheme(exportTheme({ ...copy, radius: 99, fontScale: 3 }))
    expect(parsed?.name).toBe('My Nord')
    expect(parsed?.colors).toEqual(BUILTIN_THEMES[2].colors)
    expect((parsed?.radius ?? 0) <= 18 && (parsed?.fontScale ?? 0) <= 1.25).toBe(true)
    expect(parseTheme('{"name":"x","colors":{"bg":"red"}}')).toBeNull()
    expect(parseTheme('not json')).toBeNull()
  })

  it('migrates the old System / Dark / Light setting', () => {
    localStorage.setItem('fjord.theme', 'light')
    expect(loadPrefs()).toMatchObject({ mode: 'fixed', fixed: 'fjord-light' })
  })

  it('applies a theme as CSS variables', () => {
    const root = document.createElement('div')
    applyThemeDef({ ...BUILTIN_THEMES[1], radius: 6, density: 'compact' }, root)
    expect(root.style.getPropertyValue('--bg')).toBe('#f5f6f8')
    expect(root.style.getPropertyValue('--radius')).toBe('6px')
    expect(root.dataset.theme).toBe('light')
    expect(root.dataset.density).toBe('compact')
  })
})
