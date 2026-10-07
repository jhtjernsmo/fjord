// Markdown with live [[wiki link]] chips for projects, tasks and notes.
import { useEffect, useMemo, useState } from 'react'
import Markdown, { defaultUrlTransform } from 'react-markdown'
import { CircleCheck, FilePlus2, FolderKanban, NotebookText, Square } from 'lucide-react'
import { api } from '../api'
import type { LinkTarget } from '../api'
import { useApp } from '../data'
import { useNav } from '../nav'

const LINK_RE = /\[\[([^\]\n]+?)\]\]/g
const SCHEME = 'fjord-link:'

/** Same rules as the core parser: target before an optional |label. */
export function linkTargets(markdown: string): string[] {
  const out: string[] = []
  for (const m of markdown.matchAll(LINK_RE)) {
    const target = m[1].split('|')[0].trim()
    if (target && !out.includes(target)) out.push(target)
  }
  return out
}

/** Turns [[x|label]] into a markdown link with our own scheme. */
function toMarkdownLinks(markdown: string): string {
  return markdown.replace(LINK_RE, (_, inner: string) => {
    const [target, label] = inner.split('|').map((s) => s.trim())
    return `[${(label || target).replace(/[[\]]/g, '')}](${SCHEME}${encodeURIComponent(target)})`
  })
}

function LinkChip({ target, label, resolved }: { target: string; label: string; resolved: LinkTarget | null | undefined }) {
  const nav = useNav()
  const { t } = useApp()
  if (resolved === undefined) return <span className="link-chip loading">{label}</span>
  if (resolved === null) {
    return (
      <button className="link-chip missing" title={t('notes.createLinked', { name: target })} onClick={() => nav.createNote(target)}>
        <FilePlus2 size={12} /> {label}
      </button>
    )
  }
  const open = () => {
    if (resolved.kind === 'project') nav.openProject(resolved.id)
    else if (resolved.kind === 'task' && resolved.project_id) nav.openTask(resolved.project_id, resolved.id)
    else if (resolved.kind === 'note') nav.openNote(resolved.id, resolved.project_id)
  }
  const Icon = resolved.kind === 'project' ? FolderKanban : resolved.kind === 'note' ? NotebookText : resolved.done ? CircleCheck : Square
  return (
    <button className={`link-chip kind-${resolved.kind} ${resolved.done ? 'done' : ''}`} onClick={open} title={resolved.label}>
      <Icon size={12} /> {label}
    </button>
  )
}

export function NoteMarkdown({ markdown }: { markdown: string }) {
  const { version } = useApp()
  const targets = useMemo(() => linkTargets(markdown), [markdown])
  const [resolved, setResolved] = useState<Record<string, LinkTarget | null>>({})

  useEffect(() => {
    if (targets.length === 0) return
    let cancelled = false
    api
      .resolveLinks(targets)
      .then((list) => !cancelled && setResolved(Object.fromEntries(targets.map((t, i) => [t, list[i]]))))
      .catch(() => undefined)
    return () => {
      cancelled = true
    }
  }, [targets, version])

  return (
    <Markdown
      urlTransform={(url) => (url.startsWith(SCHEME) ? url : defaultUrlTransform(url))}
      components={{
        a: ({ href, children }) => {
          if (href?.startsWith(SCHEME)) {
            const target = decodeURIComponent(href.slice(SCHEME.length))
            return <LinkChip target={target} label={String(children)} resolved={target in resolved ? resolved[target] : undefined} />
          }
          return (
            <a href={href} target="_blank" rel="noreferrer">
              {children}
            </a>
          )
        },
      }}
    >
      {toMarkdownLinks(markdown)}
    </Markdown>
  )
}
