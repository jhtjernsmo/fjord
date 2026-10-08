// Azure Boards import: Settings section (which Azure project goes into which Fjord
// project) and the background runner that imports every few minutes.
import { useEffect, useRef, useState } from 'react'
import { Download, Plus, X } from 'lucide-react'
import { api, errorMessage } from '../api'
import type { ImportMapping, ImportSettings } from '../api'
import { useApp, useLive } from '../data'

const IMPORT_EVERY_MS = 10 * 60 * 1000
const FIRST_IMPORT_DELAY_MS = 8000

/** Runs the import on startup and every 10 minutes while it's enabled. */
export function AzureImportRunner() {
  const { t, toast } = useApp()
  const [settings] = useLive(() => api.getImportSettings(), [])
  const enabled = !!settings?.enabled && settings.mappings.length > 0

  useEffect(() => {
    if (!enabled) return
    const run = () =>
      api
        .runAzureImport()
        .then((r) => r.created.length > 0 && toast(t('import.created', { n: r.created.length }), 'success'))
        .catch(() => undefined) // offline or token expired; Settings shows the error on "Import now"
    const first = window.setTimeout(run, FIRST_IMPORT_DELAY_MS)
    const every = window.setInterval(run, IMPORT_EVERY_MS)
    return () => {
      window.clearTimeout(first)
      window.clearInterval(every)
    }
  }, [enabled, t, toast])
  return null
}

const EMPTY: ImportSettings = { enabled: false, mappings: [] }

export function AzureImportSettings() {
  const { t, toast, run } = useApp()
  const [projects] = useLive(() => api.listProjects(false), [])
  const [saved] = useLive(() => api.getImportSettings(), [])
  const [settings, setSettings] = useState<ImportSettings>(EMPTY)
  const [busy, setBusy] = useState(false)

  // Load the saved settings once. Later reloads must not replace the form, or a
  // half-filled row (saved without it, since it's incomplete) would vanish mid-edit.
  const loaded = useRef(false)
  useEffect(() => {
    if (!saved || loaded.current) return
    loaded.current = true
    setSettings(saved)
  }, [saved])

  const save = (next: ImportSettings) => {
    setSettings(next)
    const complete = { ...next, mappings: next.mappings.filter((m) => m.org.trim() && m.project.trim() && m.fjord_project_id) }
    run(api.setImportSettings(complete))
  }
  const setMapping = (i: number, patch: Partial<ImportMapping>) =>
    setSettings({ ...settings, mappings: settings.mappings.map((m, j) => (j === i ? { ...m, ...patch } : m)) })

  const importNow = async () => {
    setBusy(true)
    try {
      const r = await api.runAzureImport()
      toast(t('import.report', { created: r.created.length, updated: r.updated, unmapped: r.unmapped }), 'success')
    } catch (e) {
      toast(errorMessage(e), 'error')
    } finally {
      setBusy(false)
    }
  }

  const defaultProject = projects?.[0]?.id ?? 0
  return (
    <div className="row">
      <span>
        {t('import.title')} <span className="beta">beta</span>
      </span>
      <label className="toggle-label">
        <input type="checkbox" checked={settings.enabled} onChange={(e) => save({ ...settings, enabled: e.target.checked })} />
        {t('import.enabled')}
      </label>
      {settings.mappings.map((m, i) => (
        <div className="user-row import-mapping" key={i}>
          <input className="input mono" value={m.org} placeholder={t('azure.org')} onChange={(e) => setMapping(i, { org: e.target.value })} onBlur={() => save(settings)} aria-label={t('azure.org')} />
          <input className="input grow" value={m.project} placeholder={t('import.azureProject')} onChange={(e) => setMapping(i, { project: e.target.value })} onBlur={() => save(settings)} aria-label={t('import.azureProject')} />
          <span className="dim">→</span>
          <select className="input" value={m.fjord_project_id} onChange={(e) => save({ ...settings, mappings: settings.mappings.map((x, j) => (j === i ? { ...x, fjord_project_id: Number(e.target.value) } : x)) })} aria-label={t('notes.project')}>
            {(projects ?? []).map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
          <button className="icon-btn" onClick={() => save({ ...settings, mappings: settings.mappings.filter((_, j) => j !== i) })} aria-label={t('menu.delete')}>
            <X size={14} />
          </button>
        </div>
      ))}
      <div className="user-row">
        <button className="btn" onClick={() => setSettings({ ...settings, mappings: [...settings.mappings, { org: '', project: '', fjord_project_id: defaultProject }] })}>
          <Plus size={13} /> {t('import.addMapping')}
        </button>
        <button className="btn" disabled={busy || settings.mappings.length === 0} onClick={importNow}>
          <Download size={13} /> {t('import.now')}
        </button>
      </div>
      <span className="hint">{t('import.hint')}</span>
    </div>
  )
}
