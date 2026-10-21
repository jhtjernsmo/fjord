// Overview on standup days: one line with what you did since the last standup, ready to copy or expand.
import { useState } from 'react'
import { ChevronDown, ChevronRight, Copy } from 'lucide-react'
import { api } from '../api'
import type { GitHubMention, Mention, PullRequest, ProjectSummary, Task } from '../api'
import { useApp, useLive } from '../data'
import { dateLocale } from '../i18n'
import { useNav } from '../nav'
import { localIso } from '../dueWeek'
import { memo } from '../memo'
import { buildStandup, mentionsSince, myPullRequests, previousStandup, standupText } from '../standup'
import { useMentions } from '../mentions'
import { pendingChip } from '../pending'
import type { StandupDraft, StandupEntry, Weekday } from '../standup'

/** Enough history for a week between standups. */
const ACTIVITY_LIMIT = 500

/** The names GitHub and Azure DevOps know you by, to pick out your own pull requests. */
async function myNames(orgs: string[]): Promise<string[]> {
  const [github, azure] = await Promise.all([api.githubAccount().catch(() => null), Promise.all(orgs.map((org) => api.azureAccount(org).catch(() => null)))])
  return [github?.login, ...azure.map((a) => a?.user)].filter((n): n is string => !!n)
}

interface Repos {
  /** Every project with a linked repository. */
  linked: ProjectSummary[]
  /** Azure DevOps organizations of those repositories. */
  orgs: string[]
  /** owner/repo of the GitHub ones, lowercase. */
  githubRepos: string[]
}

/** The projects' linked repositories. Local only. */
async function linkedRepos(projects: ProjectSummary[]): Promise<Repos> {
  const repos = await Promise.all(projects.map((p) => api.getProjectRepo(p.id).catch(() => null)))
  return {
    linked: projects.filter((_, i) => repos[i]),
    orgs: [...new Set(repos.map((r) => r?.azure_org).filter((o): o is string => !!o))],
    githubRepos: repos.flatMap((r) => (r?.github_owner && r.github_repo ? [`${r.github_owner}/${r.github_repo}`.toLowerCase()] : [])),
  }
}

/** GitHub mentions since the last standup; for one project, only those in its linked repository. */
async function githubMentions(since: Date, repos: string[] | null): Promise<GitHubMention[]> {
  const found = await api.githubMentions(localIso(since)).catch(() => [] as GitHubMention[])
  return repos === null ? found : found.filter((m) => repos.includes(m.repo.toLowerCase()))
}

/** The Azure DevOps projects whose work items are imported into the chosen Fjord project. */
async function azureScope(projectId: number | null): Promise<{ org: string; project: string }[] | null> {
  if (projectId === null) return null
  const settings = await api.getImportSettings().catch(() => null)
  return (settings?.mappings ?? []).filter((m) => m.fjord_project_id === projectId)
}

type LocalDraft = Pick<StandupDraft, 'done' | 'workedOn' | 'today' | 'pending'>
type NetworkDraft = Pick<StandupDraft, 'closedPrs' | 'openPrs' | 'githubMentions'>

/** Your tasks: read from the local database, so it's quick and always current. */
async function loadLocal(projects: ProjectSummary[], since: Date): Promise<LocalDraft> {
  const ids = new Set(projects.map((p) => p.id))
  const [activity, actor, boards] = await Promise.all([api.recentActivity(null, ACTIVITY_LIMIT), api.actor(), Promise.all(projects.map((p) => api.getBoard(p.id)))])
  const ownActivity = activity.filter((a) => a.project_id !== null && ids.has(a.project_id))
  return buildStandup(ownActivity, boards.flatMap((b) => b.tasks), boards.flatMap((b) => b.statuses), since, actor)
}

/** Pull requests and mentions from GitHub and Azure DevOps, all asked for at once. */
async function loadNetwork(projects: ProjectSummary[], since: Date, oneProject: boolean): Promise<NetworkDraft> {
  const repos = await linkedRepos(projects)
  const [reports, names, onGitHub] = await Promise.all([
    Promise.all(repos.linked.map((p) => api.syncPullRequests(p.id).catch(() => null))),
    myNames(repos.orgs),
    githubMentions(since, oneProject ? repos.githubRepos : null),
  ])
  const prs = reports.flatMap((r) => r?.pull_requests ?? [])
  return { ...myPullRequests(prs, names, since), githubMentions: onGitHub }
}

const NO_NETWORK: NetworkDraft = { closedPrs: [], openPrs: [], githubMentions: [] }

/** A top-level task; the subtasks you worked on fold out under it. */
function EntryItem({ entry, isDone }: { entry: StandupEntry; isDone: boolean }) {
  const { t } = useApp()
  const nav = useNav()
  const [open, setOpen] = useState(false)
  const { task, subtasks, progress } = entry
  const open_ = (target: Task) => nav.openTask(target.project_id, target.id)
  return (
    <li>
      <div className="standup-entry">
        {subtasks.length > 0 ? (
          <button className="standup-fold" onClick={() => setOpen((v) => !v)} aria-expanded={open} aria-label={t('standup.showSubtasks')} title={t('standup.showSubtasks')}>
            {open ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
          </button>
        ) : (
          <span className="standup-fold" />
        )}
        <button data-home-item data-home-action="open" onClick={() => open_(task)}>
          <span className="mono dim">#{task.id}</span> {task.title}
          {progress && <span className="standup-state"> {t('standup.subtasks', { done: progress.done, total: progress.total })}</span>}
          {!isDone && task.pending_since && <span className="standup-state"> {pendingChip(task.pending_since, t)}</span>}
          {isDone && <span className="standup-done"> {t('standup.done')}</span>}
        </button>
      </div>
      {open && (
        <ul className="standup-subtasks">
          {subtasks.map(({ task: sub, done }) => (
            <li key={sub.id}>
              <button data-home-item data-home-action="open" onClick={() => open_(sub)}>
                <span className="mono dim">#{sub.id}</span> {sub.title}
                {done && <span className="standup-done"> {t('standup.done')}</span>}
              </button>
            </li>
          ))}
        </ul>
      )}
    </li>
  )
}

function StandupList({ title, tasks, done = [], prs = [], mentions = [], githubMentions = [] }: { title: string; tasks: StandupEntry[]; done?: StandupEntry[]; prs?: PullRequest[]; mentions?: Mention[]; githubMentions?: GitHubMention[] }) {
  const { t, run } = useApp()
  const item = (entry: StandupEntry, isDone: boolean) => <EntryItem key={entry.task.id} entry={entry} isDone={isDone} />
  return (
    <div className="standup-col">
      <div className="standup-col-title">{title}</div>
      <ul>
        {done.map((task) => item(task, true))}
        {tasks.map((task) => item(task, false))}
        {prs.map((pr) => (
          <li key={pr.url}>
            <button data-home-item data-home-action="open" onClick={() => run(api.openUrl(pr.url))} title={pr.url}>
              <span className="mono dim">PR {pr.number}</span> {pr.title}
              <span className="standup-state"> {t(pr.state === 'open' ? 'standup.inReview' : pr.state === 'merged' ? 'standup.merged' : 'standup.closed')}</span>
            </button>
          </li>
        ))}
        {mentions.map((m) => (
          <li key={`${m.org}/${m.id}`}>
            <button data-home-item data-home-action="open" onClick={() => run(api.openUrl(m.url))} title={m.url}>
              <span className="mono dim">#{m.id}</span> {m.title}
              <span className="standup-state"> {t('standup.mentionedBy', { name: m.by })}</span>
            </button>
          </li>
        ))}
        {githubMentions.map((m) => (
          <li key={m.url}>
            <button data-home-item data-home-action="open" onClick={() => run(api.openUrl(m.url))} title={m.url}>
              <span className="mono dim">
                {m.repo}#{m.number}
              </span>{' '}
              {m.title}
              <span className="standup-state"> {t('standup.mentionedBy', { name: m.by })}</span>
            </button>
          </li>
        ))}
        {done.length + tasks.length + prs.length + mentions.length + githubMentions.length === 0 && <li className="dim">{t('standup.none')}</li>}
      </ul>
    </div>
  )
}

export function Standup({ projects, days, projectId }: { projects: ProjectSummary[]; days: Weekday[]; projectId: number | null }) {
  const { t, locale, toast } = useApp()
  const [open, setOpen] = useState(false)
  const [today] = useState(() => new Date())
  const since = previousStandup(today, days)
  const scope = projectId === null ? projects : projects.filter((p) => p.id === projectId)
  const scopeIds = scope.map((p) => p.id).join()
  const [local] = useLive(() => loadLocal(scope, since), [scopeIds, days.join(), today])
  // The network part is remembered for a few minutes and fills in when it arrives.
  const [network] = useLive(() => memo(`standup:${scopeIds}:${localIso(since)}`, () => loadNetwork(scope, since, projectId !== null)), [scopeIds, days.join(), today])
  const [mentionScope] = useLive(() => azureScope(projectId), [projectId])
  const allMentions = useMentions()
  if (!local) return null
  const draft: StandupDraft = { ...local, ...(network ?? NO_NETWORK), mentions: mentionScope === undefined ? [] : mentionsSince(allMentions, since, mentionScope) }
  const loadingNetwork = network === undefined
  const sinceLabel = t('standup.since', { day: since.toLocaleDateString(dateLocale(locale), { weekday: 'long' }) })
  const copy = () =>
    navigator.clipboard
      ?.writeText(standupText(draft, { since: sinceLabel, done: t('standup.done'), today: t('standup.today'), blockers: t('standup.blockers'), none: t('standup.none'), merged: t('standup.merged'), closed: t('standup.closed'), inReview: t('standup.inReview'), mentioned: t('standup.mentioned'), subtasks: (done, total) => t('standup.subtasks', { done, total }), pendingFor: (since) => pendingChip(since, t) }))
      .then(() => toast(t('standup.copied'), 'success'))
  const summary = t('standup.summary', { done: draft.done.length, worked: draft.workedOn.length, prs: loadingNetwork ? '…' : draft.closedPrs.length + draft.openPrs.length, mentions: loadingNetwork ? '…' : draft.mentions.length + draft.githubMentions.length, today: draft.today.length })
  const projectName = projectId === null ? null : scope[0]?.name
  return (
    <div className="standup">
      <div className="standup-bar">
        <button className="standup-toggle" onClick={() => setOpen((v) => !v)} aria-expanded={open}>
          {open ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          <span className="standup-name">{t('standup.title')}</span>
          {projectName && <span className="standup-project">{projectName}</span>}
          <span className="standup-summary">{summary}</span>
          {loadingNetwork && <span className="standup-loading">{t('standup.loading')}</span>}
        </button>
        <button className="btn" onClick={copy} title={t('standup.copyHint')}>
          <Copy size={13} /> {t('standup.copy')}
        </button>
      </div>
      {open && (
        <div className="standup-body">
          <StandupList title={sinceLabel} done={draft.done} tasks={draft.workedOn} prs={draft.closedPrs} mentions={draft.mentions} githubMentions={draft.githubMentions} />
          <StandupList title={t('standup.today')} tasks={draft.today} prs={draft.openPrs} />
          {draft.pending.length > 0 && <StandupList title={t('standup.blockers')} tasks={draft.pending} />}
        </div>
      )}
    </div>
  )
}
