// Settings → Appearance: pick a theme (or one for light and one for dark mode),
// and make your own with a live preview.
import { useEffect, useState } from 'react'
import { Check, ClipboardCopy, ClipboardPaste, Copy, Pencil, Trash2, TriangleAlert } from 'lucide-react'
import { useApp } from '../data'
import type { MessageKey } from '../i18n'
import { COLOR_KEYS, contrastWarnings, duplicateTheme, exportTheme, parseTheme } from '../theme'
import type { Density, ThemeColors, ThemeDef } from '../theme'
import { useThemes } from '../themes'

function Swatch({ theme }: { theme: ThemeDef }) {
  const c = theme.colors
  return (
    <div className="theme-swatch" style={{ background: c.bg, borderRadius: Math.min(theme.radius, 10) }}>
      <div className="sw-side" style={{ background: c.bg2 }} />
      <div className="sw-main">
        <div className="sw-card" style={{ background: c.panel, borderColor: c.line2, borderRadius: Math.min(theme.radius, 8) / 2 }}>
          <span style={{ background: c.text }} />
          <span style={{ background: c.muted, width: '55%' }} />
        </div>
        <div className="sw-dots">
          {[c.accent, c.ok, c.warn, c.danger].map((col, i) => (
            <i key={i} style={{ background: col }} />
          ))}
        </div>
      </div>
    </div>
  )
}

function ThemeEditor({ initial, onDone }: { initial: ThemeDef; onDone: () => void }) {
  const { t } = useApp()
  const { saveTheme, preview, prefs, setPrefs } = useThemes()
  const [draft, setDraft] = useState(initial)

  // Everything you change shows up across the app immediately.
  useEffect(() => {
    preview(draft)
  }, [draft, preview])
  useEffect(() => () => preview(null), [preview])

  const setColor = (key: keyof ThemeColors, value: string) => setDraft({ ...draft, colors: { ...draft.colors, [key]: value } })
  const warnings = contrastWarnings(draft.colors)

  const save = () => {
    const name = draft.name.trim() || t('appearance.untitled')
    saveTheme({ ...draft, name })
    setPrefs(prefs.mode === 'fixed' ? { ...prefs, fixed: draft.id } : draft.dark ? { ...prefs, dark: draft.id } : { ...prefs, light: draft.id })
    onDone()
  }

  return (
    <div className="theme-editor">
      <div className="user-row">
        <input className="input grow" value={draft.name} onChange={(e) => setDraft({ ...draft, name: e.target.value })} aria-label={t('appearance.name')} placeholder={t('appearance.name')} maxLength={40} />
        <div className="segmented">
          <button className={draft.dark ? 'on' : ''} onClick={() => setDraft({ ...draft, dark: true })}>
            {t('settings.theme.dark')}
          </button>
          <button className={!draft.dark ? 'on' : ''} onClick={() => setDraft({ ...draft, dark: false })}>
            {t('settings.theme.light')}
          </button>
        </div>
      </div>
      <div className="color-grid">
        {COLOR_KEYS.map((key) => (
          <label key={key} className="color-field">
            <input type="color" value={draft.colors[key]} onChange={(e) => setColor(key, e.target.value)} />
            <span>{t(`appearance.color.${key}` as MessageKey)}</span>
            <code>{draft.colors[key]}</code>
          </label>
        ))}
      </div>
      <div className="slider-row">
        <label>
          {t('appearance.textSize')} <span className="dim">{Math.round(draft.fontScale * 100)}%</span>
          <input type="range" min={0.85} max={1.25} step={0.05} value={draft.fontScale} onChange={(e) => setDraft({ ...draft, fontScale: Number(e.target.value) })} />
        </label>
        <label>
          {t('appearance.radius')} <span className="dim">{draft.radius}px</span>
          <input type="range" min={0} max={18} step={1} value={draft.radius} onChange={(e) => setDraft({ ...draft, radius: Number(e.target.value) })} />
        </label>
      </div>
      <div className="segmented" role="radiogroup" aria-label={t('appearance.density')}>
        {(['compact', 'cozy', 'comfortable'] as Density[]).map((d) => (
          <button key={d} className={draft.density === d ? 'on' : ''} onClick={() => setDraft({ ...draft, density: d })}>
            {t(`appearance.density.${d}` as MessageKey)}
          </button>
        ))}
      </div>
      {warnings.length > 0 && (
        <div className="hint warn contrast-warn">
          <TriangleAlert size={13} /> {t('appearance.contrast', { pairs: warnings.map((w) => `${w.pair} (${w.ratio}:1)`).join(', ') })}
        </div>
      )}
      <div className="user-row">
        <button className="btn primary" onClick={save}>
          <Check size={14} /> {t('appearance.save')}
        </button>
        <button className="btn ghost" onClick={onDone}>
          {t('dialog.cancel')}
        </button>
      </div>
    </div>
  )
}

export function ThemeSettings() {
  const { t, toast } = useApp()
  const { prefs, setPrefs, all, custom, current, deleteTheme, saveTheme } = useThemes()
  const [editing, setEditing] = useState<ThemeDef | null>(null)
  const [importing, setImporting] = useState(false)
  const [importText, setImportText] = useState('')

  if (editing) return <ThemeEditor initial={editing} onDone={() => setEditing(null)} />

  const choose = (theme: ThemeDef) =>
    setPrefs(prefs.mode === 'fixed' ? { ...prefs, fixed: theme.id } : theme.dark ? { ...prefs, dark: theme.id } : { ...prefs, light: theme.id })
  const inUse = (theme: ThemeDef) => (prefs.mode === 'fixed' ? prefs.fixed === theme.id : prefs.dark === theme.id || prefs.light === theme.id)

  const customize = (theme: ThemeDef) =>
    setEditing(theme.builtin ? duplicateTheme(theme, t('appearance.copyOf', { name: theme.name }), custom) : theme)

  const doImport = () => {
    const parsed = parseTheme(importText)
    if (!parsed) return toast(t('appearance.importInvalid'), 'error')
    const theme = { ...duplicateTheme({ ...parsed, id: '', builtin: false }, parsed.name, custom) }
    saveTheme(theme)
    setImporting(false)
    setImportText('')
    toast(t('appearance.imported', { name: theme.name }), 'success')
  }

  return (
    <div className="appearance">
      <div className="row">
        {t('appearance.mode')}
        <div className="segmented">
          <button className={prefs.mode === 'system' ? 'on' : ''} onClick={() => setPrefs({ ...prefs, mode: 'system' })}>
            {t('appearance.followSystem')}
          </button>
          <button className={prefs.mode === 'fixed' ? 'on' : ''} onClick={() => setPrefs({ ...prefs, mode: 'fixed', fixed: current.id })}>
            {t('appearance.alwaysOne')}
          </button>
        </div>
        {prefs.mode === 'system' && <span className="hint">{t('appearance.systemHint')}</span>}
      </div>

      <div className="theme-grid">
        {all.map((theme) => (
          <div key={theme.id} className={`theme-card ${inUse(theme) ? 'on' : ''}`}>
            <button className="theme-pick" onClick={() => choose(theme)} title={t('appearance.use')}>
              <Swatch theme={theme} />
              <span className="theme-name">
                {theme.name}
                {prefs.mode === 'system' && inUse(theme) && <span className="dim"> · {theme.dark ? t('settings.theme.dark') : t('settings.theme.light')}</span>}
              </span>
            </button>
            <div className="theme-actions">
              <button className="icon-btn" onClick={() => customize(theme)} title={theme.builtin ? t('appearance.duplicate') : t('appearance.edit')} aria-label={theme.builtin ? t('appearance.duplicate') : t('appearance.edit')}>
                {theme.builtin ? <Copy size={13} /> : <Pencil size={13} />}
              </button>
              <button
                className="icon-btn"
                onClick={() => navigator.clipboard?.writeText(exportTheme(theme)).then(() => toast(t('appearance.copied'), 'success'))}
                title={t('appearance.export')}
                aria-label={t('appearance.export')}
              >
                <ClipboardCopy size={13} />
              </button>
              {!theme.builtin && (
                <button className="icon-btn danger-icon" onClick={() => deleteTheme(theme.id)} title={t('menu.delete')} aria-label={t('menu.delete')}>
                  <Trash2 size={13} />
                </button>
              )}
            </div>
          </div>
        ))}
      </div>

      <div className="user-row">
        <button className="btn" onClick={() => customize(current)}>
          <Pencil size={13} /> {t('appearance.newFromCurrent')}
        </button>
        <button className="btn ghost" onClick={() => setImporting(!importing)}>
          <ClipboardPaste size={13} /> {t('appearance.import')}
        </button>
      </div>
      {importing && (
        <div className="row">
          <textarea className="md-editor import-box" value={importText} onChange={(e) => setImportText(e.target.value)} placeholder={t('appearance.importHint')} spellCheck={false} />
          <button className="btn primary" disabled={!importText.trim()} onClick={doImport}>
            {t('appearance.import')}
          </button>
        </div>
      )}

      <label className="toggle-label">
        <input type="checkbox" checked={prefs.projectAccent} onChange={(e) => setPrefs({ ...prefs, projectAccent: e.target.checked })} />
        {t('appearance.projectAccent')}
      </label>
      <span className="hint">{t('appearance.cycleHint')}</span>
    </div>
  )
}
