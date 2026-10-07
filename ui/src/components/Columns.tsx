// Column header with an options menu, the "add column" control and the board filter bar.
import { useEffect, useRef, useState } from 'react'
import { ArrowLeft, ArrowRight, CircleCheck, Ellipsis, Pencil, Plus, Search, Trash2, X } from 'lucide-react'
import { api } from '../api'
import type { Status } from '../api'
import { useApp } from '../data'
import type { MessageKey } from '../i18n'

const COLUMN_COLORS = ['#8b8f98', '#7c9cff', '#00d4b0', '#3fb950', '#f5a524', '#ff7a1a', '#ff6b6b', '#e86bff']

interface HeaderProps {
  status: Status
  count: number
  index: number
  columnCount: number
}

export function ColumnHeader({ status, count, index, columnCount }: HeaderProps) {
  const { run, t } = useApp()
  const [menu, setMenu] = useState(false)
  const [editing, setEditing] = useState(false)
  const [name, setName] = useState(status.name)
  const menuRef = useRef<HTMLDivElement>(null)

  useEffect(() => setName(status.name), [status.name])
  useEffect(() => {
    if (!menu) return
    const close = (e: MouseEvent) => !menuRef.current?.contains(e.target as Node) && setMenu(false)
    window.addEventListener('mousedown', close)
    return () => window.removeEventListener('mousedown', close)
  }, [menu])

  const rename = () => {
    setEditing(false)
    if (name.trim() && name !== status.name) run(api.updateStatus(status.id, { name: name.trim() }))
    else setName(status.name)
  }
  const act = (fn: () => void) => () => {
    setMenu(false)
    fn()
  }

  return (
    <header className="column-head">
      <span className="dot" style={{ background: status.color }} />
      {editing ? (
        <input
          className="column-rename"
          autoFocus
          value={name}
          onChange={(e) => setName(e.target.value)}
          onBlur={rename}
          onKeyDown={(e) => {
            if (e.key === 'Enter') e.currentTarget.blur()
            if (e.key === 'Escape') {
              setName(status.name)
              setEditing(false)
            }
          }}
          aria-label={t('column.name')}
        />
      ) : (
        <span className="column-name" onDoubleClick={() => setEditing(true)}>
          {status.name}
          {status.is_done && <CircleCheck size={12} className="done-mark" aria-label={t('column.doneToggle')} />}
        </span>
      )}
      <span className="count">{count}</span>
      <div className="column-menu-wrap" ref={menuRef}>
        <button className="icon-btn" onClick={() => setMenu((m) => !m)} aria-label={t('column.menu')} aria-expanded={menu}>
          <Ellipsis size={15} />
        </button>
        {menu && (
          <div className="menu" role="menu">
            <button role="menuitem" onClick={act(() => setEditing(true))}>
              <Pencil size={13} /> {t('column.rename')}
            </button>
            <div className="menu-swatches">
              {COLUMN_COLORS.map((c) => (
                <button
                  key={c}
                  className={`swatch sm ${c === status.color ? 'on' : ''}`}
                  style={{ background: c }}
                  onClick={act(() => run(api.updateStatus(status.id, { color: c })))}
                  aria-label={c}
                />
              ))}
            </div>
            <button role="menuitemcheckbox" aria-checked={status.is_done} onClick={act(() => run(api.updateStatus(status.id, { is_done: !status.is_done })))}>
              <CircleCheck size={13} /> {t('column.doneToggle')} {status.is_done ? '✓' : ''}
            </button>
            <button role="menuitem" disabled={index === 0} onClick={act(() => run(api.moveStatus(status.id, index - 1)))}>
              <ArrowLeft size={13} /> {t('column.moveLeft')}
            </button>
            <button role="menuitem" disabled={index === columnCount - 1} onClick={act(() => run(api.moveStatus(status.id, index + 1)))}>
              <ArrowRight size={13} /> {t('column.moveRight')}
            </button>
            <button role="menuitem" className="danger" disabled={count > 0 || columnCount === 1} onClick={act(() => run(api.deleteStatus(status.id)))}>
              <Trash2 size={13} /> {t('column.delete')}
            </button>
          </div>
        )}
      </div>
    </header>
  )
}

export function AddColumn({ projectId }: { projectId: number }) {
  const { run, t } = useApp()
  const [open, setOpen] = useState(false)
  const [name, setName] = useState('')
  if (!open) {
    return (
      <button className="add-column" onClick={() => setOpen(true)}>
        <Plus size={14} /> {t('column.add').replace('+ ', '')}
      </button>
    )
  }
  const submit = () => {
    if (name.trim()) run(api.createStatus(projectId, name.trim(), null, false))
    setName('')
    setOpen(false)
  }
  return (
    <form
      className="add-column open"
      onSubmit={(e) => {
        e.preventDefault()
        submit()
      }}
    >
      <input
        autoFocus
        value={name}
        onChange={(e) => setName(e.target.value)}
        onBlur={submit}
        onKeyDown={(e) => e.key === 'Escape' && (setName(''), setOpen(false))}
        placeholder={t('column.name')}
        aria-label={t('column.name')}
      />
    </form>
  )
}

export interface BoardFilter {
  text: string
  minPriority: number
  agentOnly: boolean
}

export const EMPTY_FILTER: BoardFilter = { text: '', minPriority: 0, agentOnly: false }

export function isFiltering(f: BoardFilter): boolean {
  return f.text.trim() !== '' || f.minPriority > 0 || f.agentOnly
}

interface FilterProps {
  filter: BoardFilter
  onChange: (f: BoardFilter) => void
  shown: number
  total: number
  inputRef: React.RefObject<HTMLInputElement | null>
}

export function FilterBar({ filter, onChange, shown, total, inputRef }: FilterProps) {
  const { t } = useApp()
  return (
    <div className="filter-bar">
      <label className="filter-search">
        <Search size={14} />
        <input
          ref={inputRef}
          value={filter.text}
          onChange={(e) => onChange({ ...filter, text: e.target.value })}
          onKeyDown={(e) => {
            if (e.key === 'Escape' || e.key === 'Enter') e.currentTarget.blur()
          }}
          placeholder={t('filter.placeholder')}
          aria-label={t('filter.placeholder')}
        />
        <kbd>/</kbd>
      </label>
      <select
        className="input"
        value={filter.minPriority}
        onChange={(e) => onChange({ ...filter, minPriority: Number(e.target.value) })}
        aria-label={t('task.priority')}
      >
        <option value={0}>{t('filter.anyPriority')}</option>
        {[1, 2, 3].map((p) => (
          <option key={p} value={p}>
            {t('filter.minPriority', { p: t(`priority.${p}` as MessageKey) })}
          </option>
        ))}
      </select>
      <button className={`toggle ${filter.agentOnly ? 'on' : ''}`} onClick={() => onChange({ ...filter, agentOnly: !filter.agentOnly })} aria-pressed={filter.agentOnly}>
        <span className="agent-tag">AI</span> {t('filter.agentOnly')}
      </button>
      {isFiltering(filter) && (
        <>
          <span className="filter-count">{t('filter.count', { shown, total })}</span>
          <button className="btn ghost" onClick={() => onChange(EMPTY_FILTER)}>
            <X size={13} /> {t('filter.clear')}
          </button>
        </>
      )}
    </div>
  )
}
