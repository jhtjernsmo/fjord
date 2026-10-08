// Azure Boards import: Settings section (which Azure project goes into which Fjord
// project) and the background runner that imports every few minutes.
import { useEffect, useRef, useState } from 'react'
import { Download, Plus, X } from 'lucide-react'
import { api, errorMessage } from '../api'
import type { ImportMapping, ImportReport, ImportSettings, Task } from '../api'
import type { MessageKey } from '../i18n'
import { useApp, useLive } from '../data'

const IMPORT_EVERY_MS = 10 * 60 * 1000
const FIRST_IMPORT_DELAY_MS = 8000
const NOTIFY_KEY = 'fjord.importNotify'
const TITLES_IN_NOTIFICATION = 3

export function importNotificationsOn(): boolean {
  try {
    return localStorage.getItem(NOTIFY_KEY) !== 'off'
  } catch {
    return true
  }
}

function setImportNotifications(on: boolean): void {
  try {
    localStorage.setItem(NOTIFY_KEY, on ? 'on' : 'off')
  } catch {
    /* non-fatal */
  }
}

/** "A, B, C and 2 more" */
export function listTitles(tasks: Task[], more: (n: number) => string): string {
  const shown = tasks.slice(0, TITLES_IN_NOTIFICATION).map((t) => t.title)
  const rest = tasks.length - shown.length
  return rest > 0 ? `${shown.join(', ')} ${more(rest)}` : shown.join(', ')
}

/** A desktop notification, asking for permission the first time. Never throws. */
async function notify(title: string, body: string): Promise<void> {
  try {
    const n = await import('@tauri-apps/plugin-notification')
    let granted = await n.isPermissionGranted()
    if (!granted) granted = (await n.requestPermission()) === 'granted'
    if (granted) n.sendNotification({ title, body })
  } catch {
    /* no notification support (e.g. a browser); the in-app toast still shows */
  }
}

/** Notifies about new and changed work items; says nothing when nothing changed. */
export function announceImport(report: ImportReport, t: (key: MessageKey, vars?: Record<string, string | number>) => string, toast: (text: string, kind?: 'success' | 'error') => void): void {
  const more = (n: number) => t('import.andMore', { n })
  const messages: [string, string][] = []
  if (report.created.length > 0) messages.push([t('import.created', { n: report.created.length }), listTitles(report.created, more)])
  if (report.updated_tasks.length > 0) messages.push([t('import.updated', { n: report.updated_tasks.length }), listTitles(report.updated_tasks, more)])
  for (const [title, body] of messages) {
    toast(`${title}: ${body}`, 'success')
    if (importNotificationsOn()) void notify(title, body)
  }
}

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
        .then((r) => announceImport(r, t, toast))
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
  const [notifyOn, setNotifyOn] = useState(importNotificationsOn)

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
      toast(t('import.report', { created: r.created.length, updated: r.updated, closed: r.closed, unmapped: r.unmapped }), 'success')
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
      <label className="toggle-label">
        <input
          type="checkbox"
          checked={notifyOn}
          onChange={(e) => {
            setNotifyOn(e.target.checked)
            setImportNotifications(e.target.checked)
          }}
        />
        {t('import.notify')}
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
