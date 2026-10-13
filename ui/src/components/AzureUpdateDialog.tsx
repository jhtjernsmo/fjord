// Beta: send an analysis's acceptance criteria and risk to its Azure Boards work item.
// Shows what each field holds now and what it would become; nothing is sent until confirmed.
import { useEffect, useState } from 'react'
import { ExternalLink } from 'lucide-react'
import { api, errorMessage } from '../api'
import type { AzureUpdatePreview } from '../api'
import { useApp } from '../data'
import { NoteMarkdown } from './NoteMarkdown'

export function AzureUpdateDialog({ noteId, onClose }: { noteId: number; onClose: () => void }) {
  const { t, run, toast } = useApp()
  const [preview, setPreview] = useState<AzureUpdatePreview | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [chosen, setChosen] = useState<Set<string>>(new Set())
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    let cancelled = false
    api
      .azureUpdatePreview(noteId)
      .then((p) => {
        if (cancelled) return
        setPreview(p)
        setChosen(new Set(p.changes.map((c) => c.field)))
      })
      .catch((e) => !cancelled && setError(errorMessage(e)))
    return () => {
      cancelled = true
    }
  }, [noteId])

  const toggle = (field: string) =>
    setChosen((set) => {
      const next = new Set(set)
      if (next.has(field)) next.delete(field)
      else next.add(field)
      return next
    })

  const send = async () => {
    if (!preview) return
    setBusy(true)
    const changes = preview.changes.filter((c) => chosen.has(c.field))
    const rev = await run(api.azureUpdateApply(noteId, preview.rev, changes))
    setBusy(false)
    if (rev === undefined) return
    toast(t('azureUpdate.done', { id: preview.id }), 'success')
    onClose()
  }

  return (
    <div className="overlay" onMouseDown={onClose}>
      <div className="dialog azure-update" role="dialog" aria-label={t('azureUpdate.title')} onMouseDown={(e) => e.stopPropagation()}>
        <div className="dialog-title">
          <h2>{t('azureUpdate.title')}</h2>
          <span className="beta">beta</span>
          {preview && (
            <button className="icon-btn" onClick={() => run(api.openUrl(preview.url))} title={preview.url} aria-label={t('azureUpdate.open')}>
              <ExternalLink size={14} />
            </button>
          )}
        </div>
        {error && <p className="azure-update-error">{error}</p>}
        {!preview && !error && <p className="dim">{t('azureUpdate.loading')}</p>}
        {preview && (
          <>
            <p className="dim">{t('azureUpdate.intro', { id: preview.id })}</p>
            {preview.changes.length === 0 && <p>{t('azureUpdate.nothing')}</p>}
            {preview.changes.map((c) => (
              <label key={c.field} className="azure-change">
                <span className="azure-change-head">
                  <input type="checkbox" checked={chosen.has(c.field)} onChange={() => toggle(c.field)} />
                  <strong>{c.label}</strong>
                  <span className="dim mono">{c.field}</span>
                </span>
                <span className="azure-change-cols">
                  <span>
                    <span className="azure-change-label">{t('azureUpdate.now')}</span>
                    <span className="markdown azure-change-old">{c.current_md ? <NoteMarkdown markdown={c.current_md} /> : <em className="dim">{t('azureUpdate.empty')}</em>}</span>
                  </span>
                  <span>
                    <span className="azure-change-label">{t('azureUpdate.after')}</span>
                    <span className="markdown azure-change-new">
                      <NoteMarkdown markdown={c.new_md} />
                    </span>
                  </span>
                </span>
              </label>
            ))}
            {preview.skipped.map((s) => (
              <p key={s} className="hint">
                {s}
              </p>
            ))}
          </>
        )}
        <div className="actions">
          <button className="btn ghost" onClick={onClose}>
            {t('dialog.cancel')}
          </button>
          <button className="btn primary" disabled={!preview || chosen.size === 0 || busy} onClick={send}>
            {t('azureUpdate.send')}
          </button>
        </div>
      </div>
    </div>
  )
}
