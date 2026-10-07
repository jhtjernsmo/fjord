import { useEffect, useState } from 'react'
import { api } from '../api'
import type { NewProject } from '../api'
import { useApp } from '../data'
import { LOCALES } from '../i18n'
import type { MessageKey } from '../i18n'
import { X } from 'lucide-react'
import { PROJECT_GLYPHS, ProjectGlyph } from './Icons'

const COLORS = ['#7c9cff', '#00d4b0', '#3fb950', '#f5a524', '#ff7a1a', '#ff6b6b', '#e86bff', '#a78bfa']

interface Props {
  onCancel: () => void
  onCreate: (input: NewProject) => void
}

export function NewProjectDialog({ onCancel, onCreate }: Props) {
  const { t, locale } = useApp()
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [color, setColor] = useState(COLORS[0])
  const [icon, setIcon] = useState(PROJECT_GLYPHS[0])

  const submit = () => name.trim() && onCreate({ name: name.trim(), description, color, icon, locale })

  return (
    <div className="overlay" onMouseDown={onCancel}>
      <form
        className="dialog"
        onMouseDown={(e) => e.stopPropagation()}
        onSubmit={(e) => {
          e.preventDefault()
          submit()
        }}
        onKeyDown={(e) => e.key === 'Escape' && onCancel()}
        role="dialog"
        aria-label={t('dialog.newProject')}
      >
        <h2 className="dialog-title">
          <ProjectGlyph glyph={icon} color={color} size="lg" /> {name.trim() || t('dialog.newProject')}
        </h2>
        <label className="row">
          {t('dialog.name')}
          <input className="input" autoFocus value={name} onChange={(e) => setName(e.target.value)} placeholder={t('dialog.namePlaceholder')} />
        </label>
        <label className="row">
          {t('dialog.description')}
          <input className="input" value={description} onChange={(e) => setDescription(e.target.value)} placeholder={t('dialog.optional')} />
        </label>
        <div className="row">
          {t('dialog.icon')}
          <div className="glyph-picker">
            {PROJECT_GLYPHS.map((g) => (
              <button type="button" key={g} className={`glyph-option ${g === icon ? 'on' : ''}`} onClick={() => setIcon(g)} aria-label={g}>
                {g}
              </button>
            ))}
            <input
              className="input glyph-custom"
              maxLength={3}
              value={PROJECT_GLYPHS.includes(icon) ? '' : icon}
              onChange={(e) => setIcon(e.target.value || PROJECT_GLYPHS[0])}
              placeholder="abc"
              aria-label={t('dialog.icon')}
            />
          </div>
        </div>
        <div className="row">
          {t('dialog.color')}
          <div className="swatches">
            {COLORS.map((c) => (
              <button
                type="button"
                key={c}
                className={`swatch ${c === color ? 'on' : ''}`}
                style={{ background: c }}
                onClick={() => setColor(c)}
                aria-label={c}
              />
            ))}
          </div>
        </div>
        <div className="actions">
          <button type="button" className="btn ghost" onClick={onCancel}>
            {t('dialog.cancel')}
          </button>
          <button type="submit" className="btn primary" disabled={!name.trim()}>
            {t('dialog.create')} <kbd>⏎</kbd>
          </button>
        </div>
      </form>
    </div>
  )
}

export type Theme = 'system' | 'dark' | 'light'
export const THEMES: Theme[] = ['system', 'dark', 'light']

export function SettingsDialog({ theme, onTheme, onClose }: { theme: Theme; onTheme: (t: Theme) => void; onClose: () => void }) {
  const { t, locale, setLocale, run, toast, refresh } = useApp()
  const [paths, setPaths] = useState<{ data: string; keymap: string } | null>(null)
  const [userName, setUserName] = useState('')
  const [savedName, setSavedName] = useState('')
  const [rewrite, setRewrite] = useState(true)
  useEffect(() => {
    api.actor().then((a) => {
      setUserName(a)
      setSavedName(a)
    })
  }, [])
  const saveName = async () => {
    const name = await run(api.renameUser(userName, rewrite))
    if (name) {
      setSavedName(name)
      toast(t('settings.saved'), 'success')
      refresh()
    }
  }
  useEffect(() => {
    api.dataPaths().then(setPaths).catch(() => setPaths(null))
  }, [])
  return (
    <div className="overlay" onMouseDown={onClose}>
      <div className="dialog" onMouseDown={(e) => e.stopPropagation()} role="dialog" aria-label={t('settings.title')}>
        <h2 className="dialog-title">
          {t('settings.title')}
          <button className="icon-btn push-right" onClick={onClose} aria-label={t('task.close')}>
            <X size={16} />
          </button>
        </h2>
        <div className="row">
          {t('settings.user')}
          <form
            className="user-row"
            onSubmit={(e) => {
              e.preventDefault()
              if (userName.trim() && userName.trim() !== savedName) saveName()
            }}
          >
            <input className="input grow" value={userName} onChange={(e) => setUserName(e.target.value)} aria-label={t('settings.user')} />
            <button className="btn primary" disabled={!userName.trim() || userName.trim() === savedName}>
              {t('settings.save')}
            </button>
          </form>
          <label className="toggle-label">
            <input type="checkbox" checked={rewrite} onChange={(e) => setRewrite(e.target.checked)} />
            {t('settings.rewrite')}
          </label>
          <span className="hint">{t('settings.userHint')}</span>
        </div>
        <div className="row">
          {t('settings.theme')}
          <div className="segmented">
            {THEMES.map((th) => (
              <button key={th} className={theme === th ? 'on' : ''} onClick={() => onTheme(th)}>
                {t(`settings.theme.${th}` as MessageKey)}
              </button>
            ))}
          </div>
        </div>
        <div className="row">
          {t('settings.language')}
          <div className="segmented">
            {LOCALES.map((l) => (
              <button key={l.id} className={locale === l.id ? 'on' : ''} onClick={() => setLocale(l.id)}>
                {l.label}
              </button>
            ))}
          </div>
        </div>
        <div className="row">
          {t('settings.keys')}
          <span className="hint">{t('settings.keysHint')}</span>
          {paths && <code className="path">{paths.keymap}</code>}
        </div>
        <div className="row">
          {t('settings.agents')}
          <span className="hint mono">{t('settings.agentsHint')}</span>
        </div>
        <div className="row">
          {t('settings.data')}
          <span className="hint">{t('settings.dataHint')}</span>
          {paths && <code className="path">{paths.data}</code>}
        </div>
      </div>
    </div>
  )
}

const THEME_KEY = 'fjord.theme'

export function loadTheme(): Theme {
  try {
    const v = localStorage.getItem(THEME_KEY)
    return v === 'dark' || v === 'light' ? v : 'system'
  } catch {
    return 'system'
  }
}

export function applyTheme(theme: Theme): void {
  if (theme === 'system') delete document.documentElement.dataset.theme
  else document.documentElement.dataset.theme = theme
  try {
    localStorage.setItem(THEME_KEY, theme)
  } catch {
    /* non-fatal */
  }
}
