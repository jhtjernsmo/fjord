import { useEffect, useState } from 'react'
import { FolderGit2, RefreshCw } from 'lucide-react'
import type { NewProject, Project } from '../api'
import { useApp } from '../data'
import { PROJECT_GLYPHS, ProjectGlyph } from './Icons'
import { NewRepoFields } from './NewRepoFields'
import type { RepoDraft } from './NewRepoFields'
import { appVersion, installedFromStore, setUpdateCheckEnabled, updateCheckEnabled, useUpdater } from './Updater'

const COLORS = ['#7c9cff', '#00d4b0', '#3fb950', '#f5a524', '#ff7a1a', '#ff6b6b', '#e86bff', '#a78bfa']

interface Props {
  onCancel: () => void
  /** Gets the GitHub repository to create too, when the user asked for one. */
  onCreate: (input: NewProject, repo: RepoDraft | null) => void | Promise<void>
  /** Editing an existing project instead of creating one. */
  project?: Project
}

/** Create a project, or edit an existing one's name, description, icon and colour. */
export function NewProjectDialog({ onCancel, onCreate, project }: Props) {
  const { t, locale } = useApp()
  const [name, setName] = useState(project?.name ?? '')
  const [description, setDescription] = useState(project?.description ?? '')
  const [color, setColor] = useState(project?.color ?? COLORS[0])
  const [icon, setIcon] = useState(project?.icon ?? PROJECT_GLYPHS[0])
  const colors = COLORS.includes(color) ? COLORS : [...COLORS, color]
  const [withRepo, setWithRepo] = useState(false)
  const [repo, setRepo] = useState<RepoDraft | null>(null)
  const [busy, setBusy] = useState(false)
  const canSubmit = !!name.trim() && !busy && (!withRepo || !!repo)

  const submit = async () => {
    if (!canSubmit) return
    setBusy(true)
    try {
      await onCreate({ name: name.trim(), description, color, icon, locale }, withRepo ? repo : null)
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="overlay" onMouseDown={onCancel}>
      <form
        className="dialog"
        onMouseDown={(e) => e.stopPropagation()}
        onSubmit={(e) => {
          e.preventDefault()
          void submit()
        }}
        onKeyDown={(e) => e.key === 'Escape' && onCancel()}
        role="dialog"
        aria-label={project ? t('dialog.editProject') : t('dialog.newProject')}
      >
        <h2 className="dialog-title">
          <ProjectGlyph glyph={icon} color={color} size="lg" /> {name.trim() || (project ? t('dialog.editProject') : t('dialog.newProject'))}
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
            {colors.map((c) => (
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
        {!project && (
          <div className="row">
            <label className="toggle-label">
              <input type="checkbox" checked={withRepo} onChange={(e) => setWithRepo(e.target.checked)} />
              <FolderGit2 size={14} /> {t('newRepo.toggle')}
            </label>
            {withRepo && <NewRepoFields mode="create" projectName={name} description={description} onChange={setRepo} />}
          </div>
        )}
        <div className="actions">
          <button type="button" className="btn ghost" onClick={onCancel}>
            {t('dialog.cancel')}
          </button>
          <button type="submit" className="btn primary" disabled={!canSubmit}>
            {busy ? (
              <>
                <RefreshCw size={13} className="spin" /> {withRepo ? t('newRepo.creating') : t('dialog.create')}
              </>
            ) : (
              <>
                {project ? t('settings.save') : t('dialog.create')} <kbd>⏎</kbd>
              </>
            )}
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
  const [version, setVersion] = useState<string | null>(null)
  const [store, setStore] = useState(false)
  useEffect(() => {
    void appVersion().then(setVersion)
    void installedFromStore().then(setStore)
  }, [])
  if (store)
    return (
      <div className="row">
        {t('update.settings')}
        {version && <span className="mono dim">{t('update.installed', { version })}</span>}
        <span className="hint">{t('update.store')}</span>
      </div>
    )
  return (
    <div className="row">
      {t('update.settings')}
      {version && <span className="mono dim">{t('update.installed', { version })}</span>}
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
