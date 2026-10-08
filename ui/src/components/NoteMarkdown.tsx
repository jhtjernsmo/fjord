// Markdown with live [[wiki link]] chips for projects, tasks and notes.
import { useEffect, useMemo, useState } from 'react'
import type React from 'react'
import Markdown, { defaultUrlTransform } from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { CircleCheck, FilePlus2, FolderKanban, NotebookText, Square } from 'lucide-react'
import { api } from '../api'
import type { LinkTarget } from '../api'
import { useApp } from '../data'
import { useNav } from '../nav'
import { MermaidDiagram } from './Mermaid'

/** GitHub-flavored markdown: tables, task lists, strikethrough, autolinks. */
export const MARKDOWN_PLUGINS = [remarkGfm]

/** ```mermaid code blocks become diagrams; other code stays a code block. */
export function CodeBlock({ node, children }: { node?: { children?: unknown[] }; children?: React.ReactNode }) {
  const code = node?.children?.[0] as { tagName?: string; properties?: { className?: unknown }; children?: { value?: string }[] } | undefined
  const classes = code?.properties?.className
  const isMermaid = Array.isArray(classes) && classes.includes('language-mermaid')
  if (isMermaid) return <MermaidDiagram code={(code?.children ?? []).map((c) => c.value ?? '').join('')} />
  return <pre>{children}</pre>
}

/** Markdown overrides shared by notes and task descriptions. */
export const MARKDOWN_COMPONENTS = { table: ScrollTable, pre: CodeBlock }

/** Wide tables scroll sideways instead of stretching the page. */
export function ScrollTable({ children }: { children?: React.ReactNode }) {
  return (
    <div className="md-table">
      <table>{children}</table>
    </div>
  )
}

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
      remarkPlugins={MARKDOWN_PLUGINS}
      urlTransform={(url) => (url.startsWith(SCHEME) ? url : defaultUrlTransform(url))}
      components={{
        ...MARKDOWN_COMPONENTS,
        a: ({ href, children }) => {
          if (href?.startsWith(SCHEME)) {
            const target = decodeURIComponent(href.slice(SCHEME.length))
            return <LinkChip target={target} label={String(children)} resolved={target in resolved ? resolved[target] : undefined} />
          }
          return (
            <a href={href} rel="noreferrer">
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
