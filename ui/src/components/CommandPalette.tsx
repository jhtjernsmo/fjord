// Ctrl+K palette with three modes: commands, projects and full-text search.
import { useEffect, useMemo, useRef, useState } from 'react'
import type { ReactNode } from 'react'
import { ChevronRight, NotebookText, Paperclip, SquareCheck } from 'lucide-react'
import { api, errorMessage } from '../api'
import type { ProjectSummary, SearchHit } from '../api'
import { useApp } from '../data'
import type { MessageKey } from '../i18n'
import { DEFAULT_ACTIONS, displayKeys } from '../keymap'
import { ProjectGlyph } from './Icons'

export type PaletteMode = 'commands' | 'projects' | 'search'

interface Item {
  key: string
  icon: ReactNode
  label: string
  sub?: string
  run: () => void
}

interface Props {
  mode: PaletteMode
  projects: ProjectSummary[]
  onClose: () => void
  onAction: (actionId: string) => void
  onProject: (id: number) => void
  onHit: (hit: SearchHit) => void
}

const SEARCH_DEBOUNCE_MS = 120
const MODE_LABEL: Record<PaletteMode, MessageKey> = {
  commands: 'palette.commands',
  projects: 'palette.projects',
  search: 'palette.search',
}
const HIT_ICON: Record<SearchHit['kind'], ReactNode> = {
  task: <SquareCheck size={15} strokeWidth={1.6} />,
  note: <NotebookText size={15} strokeWidth={1.6} />,
  file: <Paperclip size={15} strokeWidth={1.6} />,
}
const HIDDEN_COMMANDS = new Set(['palette.open', 'panel.close'])

function fuzzy(text: string, query: string): boolean {
  const haystack = text.toLowerCase()
  let i = 0
  for (const ch of query.toLowerCase()) {
    i = haystack.indexOf(ch, i)
    if (i < 0) return false
    i++
  }
  return true
}

export function CommandPalette({ mode, projects, onClose, onAction, onProject, onHit }: Props) {
  const { keymap, toast, t } = useApp()
  const [query, setQuery] = useState('')
  const [hits, setHits] = useState<SearchHit[]>([])
  const [index, setIndex] = useState(0)
  const listRef = useRef<HTMLUListElement>(null)

  useEffect(() => {
    if (mode !== 'search') return
    const id = window.setTimeout(() => {
      api
        .search(query)
        .then(setHits)
        .catch((err) => toast(errorMessage(err), 'error'))
    }, SEARCH_DEBOUNCE_MS)
    return () => window.clearTimeout(id)
  }, [mode, query, toast])

  const items: Item[] = useMemo(() => {
    const projectName = (id: number) => projects.find((p) => p.id === id)?.name ?? ''
    if (mode === 'commands') {
      return DEFAULT_ACTIONS.filter((a) => a.scope === 'global' && !HIDDEN_COMMANDS.has(a.id))
        .map((a) => ({ a, label: t(`action.${a.id}` as MessageKey) }))
        .filter(({ a, label }) => fuzzy(`${t(`group.${a.group}` as MessageKey)} ${label}`, query))
        .map(({ a, label }) => ({
          key: a.id,
          icon: <ChevronRight size={15} strokeWidth={1.6} />,
          label,
          sub: displayKeys(keymap.bindings[a.id], keymap.leader),
          run: () => onAction(a.id),
        }))
    }
    if (mode === 'projects') {
      return projects
        .filter((p) => fuzzy(p.name, query))
        .map((p) => ({
          key: `p${p.id}`,
          icon: <ProjectGlyph glyph={p.icon} color={p.color} />,
          label: p.name,
          sub: `${p.done_count}/${p.task_count}`,
          run: () => onProject(p.id),
        }))
    }
    return hits.map((h) => ({
      key: `${h.kind}${h.ref_id}`,
      icon: HIT_ICON[h.kind],
      label: h.title,
      sub: `${projectName(h.project_id)}${h.snippet ? ` · ${h.snippet}` : ''}`,
      run: () => onHit(h),
    }))
  }, [mode, query, hits, projects, keymap, onAction, onProject, onHit, t])

  useEffect(() => setIndex(0), [query, mode])
  useEffect(() => {
    listRef.current?.children[index]?.scrollIntoView({ block: 'nearest' })
  }, [index])

  const choose = (item: Item | undefined) => {
    if (!item) return
    onClose()
    item.run()
  }

  return (
    <div className="overlay" onMouseDown={onClose}>
      <div className="palette" onMouseDown={(e) => e.stopPropagation()} role="dialog" aria-label={t(MODE_LABEL[mode])}>
        <div className="mode">{t(MODE_LABEL[mode])}</div>
        <input
          autoFocus
          value={query}
          placeholder={mode === 'search' ? t('palette.searchPlaceholder') : t('palette.filter')}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            const down = e.key === 'ArrowDown' || (e.ctrlKey && (e.key === 'j' || e.key === 'n'))
            const up = e.key === 'ArrowUp' || (e.ctrlKey && (e.key === 'k' || e.key === 'p'))
            if (down) setIndex((i) => Math.min(i + 1, items.length - 1))
            else if (up) setIndex((i) => Math.max(i - 1, 0))
            else if (e.key === 'Enter') choose(items[index])
            else if (e.key === 'Escape') onClose()
            else return
            e.preventDefault()
            e.stopPropagation()
          }}
        />
        <ul ref={listRef}>
          {items.map((item, i) => (
            <li key={item.key} className={i === index ? 'on' : ''} onMouseEnter={() => setIndex(i)} onClick={() => choose(item)}>
              <span className="li-icon">{item.icon}</span>
              <span className="label">{item.label}</span>
              {item.sub && <span className="sub">{item.sub}</span>}
            </li>
          ))}
        </ul>
        {items.length === 0 && (
          <div className="none">{mode === 'search' && !query ? t('palette.startTyping') : t('palette.noMatches')}</div>
        )}
      </div>
    </div>
  )
}
