// One note: title, metadata, markdown editor with [[ autocomplete, live preview and backlinks.
import { useEffect, useRef, useState } from 'react'
import { FolderKanban, NotebookText, Pin, PinOff, Square, Trash2 } from 'lucide-react'
import { api, relativeTime } from '../api'
import type { LinkTarget, Note, ProjectSummary } from '../api'
import { useActions, useApp, useLive } from '../data'
import { useNav } from '../nav'
import type { MessageKey } from '../i18n'
import { useEntityActions } from './actions'
import { NoteMarkdown } from './NoteMarkdown'

const AUTOSAVE_MS = 600
const OPEN_LINK_RE = /\[\[([^\]\n|]*)$/

/** What to insert for a suggestion: tasks by id (stable even if renamed). */
function linkText(s: LinkTarget): string {
  return s.kind === 'task' ? `#${s.id}|${s.label}` : s.label
}

export function Backlinks({ kind, id }: { kind: 'project' | 'task' | 'note'; id: number }) {
  const { t, locale } = useApp()
  const nav = useNav()
  const [notes] = useLive(() => api.backlinks(kind, id), [kind, id])
  if (!notes || notes.length === 0) return null
  return (
    <div className="backlinks">
      <div className="section-title">{t('notes.mentionedIn', { n: notes.length })}</div>
      {notes.map((n) => (
        <button key={n.id} className="backlink" onClick={() => nav.openNote(n.id, n.project_id)}>
          <NotebookText size={14} /> <span className="grow">{n.title}</span>
          <span className="dim">{relativeTime(n.updated_at, t, locale)}</span>
        </button>
      ))}
    </div>
  )
}

export function NoteEditor({ note, projects, onDeleted }: { note: Note; projects: ProjectSummary[]; onDeleted: () => void }) {
  const { run, t } = useApp()
  const { deleteNote } = useEntityActions()
  const [title, setTitle] = useState(note.title)
  const [body, setBody] = useState(note.body_md)
  const [folder, setFolder] = useState(note.folder)
  const [mode, setMode] = useState<'split' | 'edit' | 'preview'>('split')
  const [suggest, setSuggest] = useState<{ query: string; items: LinkTarget[]; index: number } | null>(null)
  const textRef = useRef<HTMLTextAreaElement>(null)
  const saved = useRef({ title: note.title, body: note.body_md })

  // Reset when switching notes.
  useEffect(() => {
    setTitle(note.title)
    setBody(note.body_md)
    setFolder(note.folder)
    saved.current = { title: note.title, body: note.body_md }
  }, [note.id]) // eslint-disable-line react-hooks/exhaustive-deps

  // Autosave the body shortly after typing stops.
  useEffect(() => {
    if (body === saved.current.body) return
    const id = window.setTimeout(() => {
      saved.current.body = body
      api.updateNote(note.id, saved.current.title, body).catch(() => undefined)
    }, AUTOSAVE_MS)
    return () => window.clearTimeout(id)
  }, [body, note.id])

  useActions(
    {
      'notes.edit': () => {
        if (mode === 'preview') setMode('split')
        window.setTimeout(() => textRef.current?.focus(), 0)
      },
      'notes.mode': () => setMode((m) => (m === 'edit' ? 'split' : m === 'split' ? 'preview' : 'edit')),
    },
    'notes',
  )

  const saveTitle = () => {
    const next = title.trim()
    if (!next || next === saved.current.title) return setTitle(saved.current.title)
    saved.current.title = next
    run(api.updateNote(note.id, next, body))
  }

  const updateSuggestions = (value: string, caret: number) => {
    const m = value.slice(0, caret).match(OPEN_LINK_RE)
    if (!m) return setSuggest(null)
    const query = m[1]
    api
      .linkSuggestions(query)
      .then((items) => setSuggest(items.length ? { query, items, index: 0 } : null))
      .catch(() => setSuggest(null))
  }

  const insert = (s: LinkTarget) => {
    const el = textRef.current
    if (!el) return
    const caret = el.selectionStart
    const before = body.slice(0, caret).replace(OPEN_LINK_RE, `[[${linkText(s)}]]`)
    const after = body.slice(caret).replace(/^[^\]\n]*\]\]/, '')
    setBody(before + after)
    setSuggest(null)
    requestAnimationFrame(() => {
      el.focus()
      el.setSelectionRange(before.length, before.length)
    })
  }

  return (
    <div className="note-editor">
      <div className="note-head">
        <input className="title-input" value={title} onChange={(e) => setTitle(e.target.value)} onBlur={saveTitle} onKeyDown={(e) => e.key === 'Enter' && e.currentTarget.blur()} aria-label={t('task.title')} />
        <button className="icon-btn" onClick={() => run(api.setNotePinned(note.id, !note.pinned))} title={note.pinned ? t('notes.unpin') : t('notes.pin')} aria-label={t('notes.pin')}>
          {note.pinned ? <PinOff size={15} /> : <Pin size={15} />}
        </button>
        <button className="icon-btn danger-icon" onClick={() => deleteNote(note).then(onDeleted)} title={t('menu.delete')} aria-label={t('menu.delete')}>
          <Trash2 size={15} />
        </button>
      </div>
      <div className="note-meta">
        <label>
          <FolderKanban size={13} />
          <select className="input" value={note.project_id ?? ''} onChange={(e) => run(api.moveNote(note.id, e.target.value ? Number(e.target.value) : null))} aria-label={t('notes.project')}>
            <option value="">{t('notes.noProject')}</option>
            {projects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
        </label>
        <input
          className="input mono"
          value={folder}
          placeholder={t('notes.folder')}
          onChange={(e) => setFolder(e.target.value)}
          onBlur={() => folder !== note.folder && run(api.setNoteFolder(note.id, folder))}
          aria-label={t('notes.folder')}
        />
        <div className="segmented" role="radiogroup">
          {(['edit', 'split', 'preview'] as const).map((m) => (
            <button key={m} className={mode === m ? 'on' : ''} onClick={() => setMode(m)}>
              {t(`notes.mode.${m}` as MessageKey)}
            </button>
          ))}
        </div>
      </div>

      <div className={`note-body mode-${mode}`}>
        {mode !== 'preview' && (
          <div className="note-input">
            <textarea
              ref={textRef}
              className="md-editor"
              value={body}
              placeholder={t('notes.placeholder')}
              onChange={(e) => {
                setBody(e.target.value)
                updateSuggestions(e.target.value, e.target.selectionStart)
              }}
              onBlur={() => window.setTimeout(() => setSuggest(null), 150)}
              onKeyDown={(e) => {
                if (!suggest) return
                if (e.key === 'ArrowDown') setSuggest({ ...suggest, index: (suggest.index + 1) % suggest.items.length })
                else if (e.key === 'ArrowUp') setSuggest({ ...suggest, index: (suggest.index - 1 + suggest.items.length) % suggest.items.length })
                else if (e.key === 'Enter' || e.key === 'Tab') insert(suggest.items[suggest.index])
                else if (e.key === 'Escape') setSuggest(null)
                else return
                e.preventDefault()
                e.stopPropagation()
              }}
            />
            {suggest && (
              <div className="link-suggest" role="listbox">
                {suggest.items.map((s, i) => (
                  <button key={`${s.kind}${s.id}`} role="option" aria-selected={i === suggest.index} className={i === suggest.index ? 'on' : ''} onMouseDown={(e) => (e.preventDefault(), insert(s))}>
                    {s.kind === 'project' ? <FolderKanban size={13} /> : s.kind === 'note' ? <NotebookText size={13} /> : <Square size={13} />}
                    <span className="grow">{s.label}</span>
                    <span className="dim mono">{s.kind === 'task' ? `#${s.id}` : t(`notes.kind.${s.kind}` as MessageKey)}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        )}
        {mode !== 'edit' && (
          <div className={`markdown note-preview ${body ? '' : 'empty'}`}>{body ? <NoteMarkdown markdown={body} /> : t('notes.empty')}</div>
        )}
      </div>
      <Backlinks kind="note" id={note.id} />
    </div>
  )
}
