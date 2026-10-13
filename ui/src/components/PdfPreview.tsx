// A note laid out as an A4 page, as it will look in a PDF. Saving goes through the system print
// dialog, which has "Save as PDF" on Linux, macOS and Windows; print styles hide everything else.
import { useEffect } from 'react'
import { createPortal } from 'react-dom'
import { FileDown, X } from 'lucide-react'
import { api } from '../api'
import type { Note } from '../api'
import { useApp } from '../data'
import { dateLocale } from '../i18n'
import { NoteMarkdown } from './NoteMarkdown'

/** Class on <html> while the preview is open, so print styles only apply then. */
const PRINTING_CLASS = 'pdf-open'

export function PdfPreview({ note, project, onClose }: { note: Note; project: string | null; onClose: () => void }) {
  const { t, locale, run } = useApp()

  useEffect(() => {
    document.documentElement.classList.add(PRINTING_CLASS)
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return
      e.preventDefault()
      e.stopImmediatePropagation()
      onClose()
    }
    window.addEventListener('keydown', onKey, true)
    return () => {
      document.documentElement.classList.remove(PRINTING_CLASS)
      window.removeEventListener('keydown', onKey, true)
    }
  }, [onClose])

  const updated = new Date(note.updated_at).toLocaleDateString(dateLocale(locale), { day: 'numeric', month: 'long', year: 'numeric' })

  return createPortal(
    <div className="pdf-overlay" role="dialog" aria-label={t('pdf.title')}>
      <div className="pdf-toolbar">
        <span className="pdf-toolbar-title">{t('pdf.title')}</span>
        <span className="dim">{t('pdf.hint')}</span>
        <span className="spacer" />
        <button className="btn primary" onClick={() => run(api.printPage())} autoFocus>
          <FileDown size={14} /> {t('pdf.save')}
        </button>
        <button className="btn ghost" onClick={onClose} aria-label={t('task.close')}>
          <X size={14} /> <kbd>Esc</kbd>
        </button>
      </div>
      <div className="pdf-scroll">
        <article className="pdf-page">
          <header className="pdf-header">
            <h1>{note.title}</h1>
            <div className="pdf-meta">{[project, note.folder, updated].filter(Boolean).join(' · ')}</div>
          </header>
          <div className="markdown pdf-body">{note.body_md ? <NoteMarkdown markdown={note.body_md} printed /> : t('notes.empty')}</div>
        </article>
      </div>
    </div>,
    document.body,
  )
}
