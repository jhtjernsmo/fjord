// Azure Boards mentions in the Overview: work items where someone @mentioned you
// but that aren't assigned to you. Open them, add one as a task, or dismiss it.
import { useEffect, useState } from 'react'
import { AtSign, ExternalLink, ListPlus, X } from 'lucide-react'
import { api, relativeTime } from '../api'
import type { Mention, ProjectSummary } from '../api'
import { useApp, useLive } from '../data'
import { dismissMention, setMentions, useMentions } from '../mentions'
import { importNotificationsOn, notify } from './AzureImport'
import { useMenus } from './Menus'
import { ProjectGlyph } from './Icons'

const CHECK_EVERY_MS = 10 * 60 * 1000
const FIRST_CHECK_DELAY_MS = 5000
/** The Overview shows the newest few; "Show all" expands the list. */
const SHOWN = 3

/** Checks for mentions on startup and every 10 minutes when Azure Boards is set up. */
export function MentionsRunner() {
  const { t } = useApp()
  const [settings] = useLive(() => api.getImportSettings(), [])
  const enabled = (settings?.mappings.length ?? 0) > 0

  useEffect(() => {
    if (!enabled) return
    const check = () =>
      api
        .azureMentions()
        .then((list) => {
          const fresh = setMentions(list)
          if (fresh.length === 0 || !importNotificationsOn()) return
          const [first] = fresh
          const title = fresh.length === 1 ? t('mentions.notifyOne', { by: first.by }) : t('mentions.notifyMany', { n: fresh.length })
          void notify(title, fresh.length === 1 ? first.title : fresh.map((m) => m.title).join(', '))
        })
        .catch(() => undefined) // offline or token expired; the import settings show the error
    const first = window.setTimeout(check, FIRST_CHECK_DELAY_MS)
    const every = window.setInterval(check, CHECK_EVERY_MS)
    return () => {
      window.clearTimeout(first)
      window.clearInterval(every)
    }
  }, [enabled, t])
  return null
}

function kindClass(kind: string): string {
  const k = kind.toLowerCase()
  if (k === 'bug') return 'kind-bug'
  if (k.includes('story') || k.includes('backlog')) return 'kind-story'
  if (k === 'feature' || k === 'epic') return 'kind-feature'
  return 'kind-task'
}

function MentionRow({ mention, projects }: { mention: Mention; projects: ProjectSummary[] }) {
  const { t, locale, run, toast } = useApp()
  const { openMenu } = useMenus()
  const [settings] = useLive(() => api.getImportSettings(), [])
  const mapped = settings?.mappings.find(
    (m) => m.org.toLowerCase() === mention.org.toLowerCase() && m.project.toLowerCase() === mention.project.toLowerCase(),
  )?.fjord_project_id
  const ordered = [...projects].sort((a, b) => Number(b.id === mapped) - Number(a.id === mapped))

  const addTo = async (project: ProjectSummary) => {
    const task = await run(api.addWorkItemTask(project.id, mention.org, mention.id))
    if (!task) return
    toast(t('mentions.added', { project: project.name }), 'success')
    dismissMention(mention)
  }
  // Anchored under the button, so it also opens in the right place from the keyboard.
  const chooseProject = (e: React.MouseEvent<HTMLElement>) => {
    const rect = e.currentTarget.getBoundingClientRect()
    openMenu(
      { clientX: rect.left, clientY: rect.bottom, preventDefault: () => e.preventDefault() },
      ordered.map((p) => ({
        label: p.name,
        icon: <ProjectGlyph glyph={p.icon} color={p.color} />,
        onSelect: () => void addTo(p),
      })),
    )
  }

  return (
    <div className="mention" data-home-item tabIndex={-1}>
      <div className="mention-main">
        <div className="mention-head">
          <span className={`mention-kind ${kindClass(mention.kind)}`}>
            {mention.kind} {mention.id}
          </span>
          <button className="mention-title" data-home-action="open" onClick={() => run(api.openUrl(mention.url))} title={mention.url}>
            {mention.title}
          </button>
        </div>
        <div className="mention-meta">
          <span className={`mention-by ${mention.snippet ? 'said' : ''}`}>{mention.by}</span>
          <span className="mention-snippet">{mention.snippet || t('mentions.noComment')}</span>
        </div>
      </div>
      <div className="mention-side">
        <span className="mention-when">
          {mention.project} · {relativeTime(mention.at, t, locale)}
        </span>
        <div className="mention-actions">
          <button className="icon-btn" data-home-action="add" onClick={chooseProject} title={t('mentions.addTask')} aria-label={t('mentions.addTask')}>
            <ListPlus size={15} />
          </button>
          <button className="icon-btn" onClick={() => run(api.openUrl(mention.url))} title={t('mentions.open')} aria-label={t('mentions.open')}>
            <ExternalLink size={14} />
          </button>
          <button className="icon-btn" data-home-action="dismiss" onClick={() => dismissMention(mention)} title={t('mentions.dismiss')} aria-label={t('mentions.dismiss')}>
            <X size={15} />
          </button>
        </div>
      </div>
    </div>
  )
}

/** The Overview section; renders nothing when there are no mentions. */
export function MentionsSection({ projects }: { projects: ProjectSummary[] }) {
  const { t } = useApp()
  const mentions = useMentions()
  const [showAll, setShowAll] = useState(false)
  if (mentions.length === 0) return null
  // Newest first, whichever organization it came from.
  const sorted = [...mentions].sort((a, b) => b.at.localeCompare(a.at))
  return (
    <div>
      <div className="section-title" style={{ marginBottom: 8 }}>
        <span className="mentions-heading">
          <AtSign size={13} /> {t('mentions.title')} <span className="count">{mentions.length}</span>
        </span>
      </div>
      <div className="mentions">
        {(showAll ? sorted : sorted.slice(0, SHOWN)).map((m) => (
          <MentionRow key={`${m.org}/${m.id}`} mention={m} projects={projects} />
        ))}
      </div>
      {mentions.length > SHOWN && (
        <button className="show-more" onClick={() => setShowAll((v) => !v)}>
          {showAll ? t('home.showLess') : t('mentions.showAll', { n: mentions.length })}
        </button>
      )}
    </div>
  )
}
