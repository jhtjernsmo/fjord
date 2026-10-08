import { useState } from 'react'
import type { NewProject } from '../api'
import { useApp } from '../data'
import { PROJECT_GLYPHS, ProjectGlyph } from './Icons'
import { setUpdateCheckEnabled, updateCheckEnabled, useUpdater } from './Updater'

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

export function UpdateSettings() {
  const { t } = useApp()
  const [auto, setAuto] = useState(updateCheckEnabled)
  const { update, checking, runCheck, install } = useUpdater()
  return (
    <div className="row">
      {t('update.settings')}
      <label className="toggle-label">
        <input
          type="checkbox"
          checked={auto}
          onChange={(e) => {
            setAuto(e.target.checked)
            setUpdateCheckEnabled(e.target.checked)
          }}
        />
        {t('update.auto')}
      </label>
      <div className="user-row">
        <button className="btn" disabled={checking} onClick={() => runCheck(true)}>
          {t('update.checkNow')}
        </button>
        {update && (
          <button className="btn primary" onClick={install}>
            {t('update.install')} ({update.version})
          </button>
        )}
      </div>
      <span className="hint">{t('update.hint')}</span>
    </div>
  )
}
