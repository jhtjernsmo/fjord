// Git tab: link a repository, branches, recent commits and GitHub pull requests.
import { useCallback, useEffect, useState, useSyncExternalStore } from 'react'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { CircleCheck, CircleDashed, CircleX, ExternalLink, FolderGit2, GitBranch, GitMerge, GitPullRequest, RefreshCw, Unlink } from 'lucide-react'
import { api, errorMessage, relativeTime } from '../api'
import type { Checks, LinkedPullRequest, ProjectRepo } from '../api'
import { useApp, useLive } from '../data'
import type { MessageKey } from '../i18n'

// ---- tiny shared cache of the last PR sync per project (used by the task panel) ----
const prCache = new Map<number, LinkedPullRequest[]>()
const listeners = new Set<() => void>()
function setPrs(projectId: number, prs: LinkedPullRequest[]) {
  prCache.set(projectId, prs)
  listeners.forEach((l) => l())
}
/** Syncs pull requests from GitHub and updates the shared cache. */
export async function refreshPullRequests(projectId: number) {
  const report = await api.syncPullRequests(projectId)
  setPrs(projectId, report.pull_requests)
  return report
}

export function usePullRequests(projectId: number): LinkedPullRequest[] | undefined {
  return useSyncExternalStore(
    (cb) => {
      listeners.add(cb)
      return () => listeners.delete(cb)
    },
    () => prCache.get(projectId),
  )
}

export function ChecksIcon({ checks }: { checks: Checks }) {
  if (checks === 'success') return <CircleCheck size={14} className="ok" aria-label="CI passed" />
  if (checks === 'failure') return <CircleX size={14} className="bad" aria-label="CI failed" />
  if (checks === 'pending') return <CircleDashed size={14} className="pending" aria-label="CI running" />
  return <span className="checks-none" />
}

export function PrStateBadge({ pr }: { pr: LinkedPullRequest }) {
  const { t } = useApp()
  const state = pr.draft && pr.state === 'open' ? 'draft' : pr.state
  const Icon = pr.state === 'merged' ? GitMerge : GitPullRequest
  return (
    <span className={`pr-state pr-${state}`}>
      <Icon size={12} /> {t(`git.state.${state}` as MessageKey)}
    </span>
  )
}

function LinkRepo({ projectId }: { projectId: number }) {
  const { run, t } = useApp()
  const [path, setPath] = useState('')
  const choose = async () => {
    const picked = await openDialog({ directory: true, multiple: false })
    if (typeof picked === 'string') setPath(picked)
  }
  return (
    <div className="page">
      <div className="git-link-card">
        <FolderGit2 size={34} strokeWidth={1.3} />
        <h3>{t('git.linkTitle')}</h3>
        <p className="hint">{t('git.linkHint')}</p>
        <form
          className="row-inline"
          onSubmit={(e) => {
            e.preventDefault()
            if (path.trim()) run(api.linkRepo(projectId, path.trim()))
          }}
        >
          <button type="button" className="btn" onClick={choose}>
            {t('git.choose')}
          </button>
          <input className="input mono grow" value={path} onChange={(e) => setPath(e.target.value)} placeholder={t('git.pathPlaceholder')} aria-label={t('git.pathPlaceholder')} />
          <button className="btn primary" disabled={!path.trim()}>
            {t('git.link')}
          </button>
        </form>
      </div>
    </div>
  )
}

export function GitView({ projectId, onOpenTask }: { projectId: number; onOpenTask: (id: number) => void }) {
  const { run, t, locale, toast } = useApp()
  const [repo] = useLive(() => api.getProjectRepo(projectId), [projectId])
  const [overview] = useLive(() => (repo ? api.gitOverview(projectId) : Promise.resolve(null)), [projectId, repo?.path])
  const prs = usePullRequests(projectId)
  const [syncing, setSyncing] = useState(false)
  const [syncError, setSyncError] = useState<string | null>(null)
  const hasGithub = !!repo?.github_owner

  const sync = useCallback(
    async (quiet = false) => {
      setSyncing(true)
      setSyncError(null)
      try {
        const report = await refreshPullRequests(projectId)
        if (report.completed.length > 0) toast(t('git.completed', { n: report.completed.length }), 'success')
        else if (!quiet) toast(t('git.synced', { n: report.pull_requests.length }), 'success')
      } catch (err) {
        setSyncError(errorMessage(err))
      } finally {
        setSyncing(false)
      }
    },
    [projectId, t, toast],
  )

  useEffect(() => {
    if (hasGithub && !prCache.has(projectId)) sync(true)
  }, [hasGithub, projectId, sync])

  if (repo === undefined) return null
  if (repo === null) return <LinkRepo projectId={projectId} />

  return (
    <div className="page git-page">
      <RepoHeader repo={repo} />
      {overview && (
        <div className="git-status">
          <GitBranch size={14} /> {t('git.onBranch', { branch: overview.current_branch || 'HEAD' })}
          {overview.dirty && <span className="chip warn-chip">{t('git.dirty')}</span>}
        </div>
      )}

      <section className="git-section">
        <div className="section-title">
          {t('git.pullRequests')}
          {hasGithub && (
            <button onClick={() => sync()} disabled={syncing}>
              <RefreshCw size={12} className={syncing ? 'spin' : ''} /> {syncing ? t('git.syncing') : t('git.sync')}
            </button>
          )}
        </div>
        {!hasGithub && <div className="hint">{t('git.noGithub')}</div>}
        {syncError && <div className="error-box">{syncError}</div>}
        {hasGithub && prs && prs.length === 0 && <div className="hint">{t('git.noPrs')}</div>}
        <div className="pr-list">
          {(prs ?? []).map((pr) => (
            <div className="pr-row" key={pr.number}>
              <ChecksIcon checks={pr.checks} />
              <span className="mono dim">#{pr.number}</span>
              <span className="pr-title">{pr.title}</span>
              <PrStateBadge pr={pr} />
              <span className="mono dim pr-branch">{pr.head}</span>
              {pr.task_id && (
                <button className="chip" onClick={() => onOpenTask(pr.task_id as number)}>
                  {t('git.task', { id: pr.task_id })}
                </button>
              )}
              <button className="icon-btn" onClick={() => run(api.openUrl(pr.url))} title={t('git.openOnGithub')} aria-label={t('git.openOnGithub')}>
                <ExternalLink size={14} />
              </button>
            </div>
          ))}
        </div>
      </section>

      {overview && (
        <div className="git-columns">
          <section className="git-section">
            <div className="section-title">{t('git.branches')}</div>
            {overview.branches.map((b) => (
              <div className={`branch-row ${b.current ? 'current' : ''}`} key={b.name}>
                <GitBranch size={13} />
                <span className="mono grow">{b.name}</span>
                {b.ahead > 0 && <span className="dim">↑{b.ahead}</span>}
                {b.behind > 0 && <span className="dim">↓{b.behind}</span>}
                <span className="mono dim">{b.sha}</span>
              </div>
            ))}
          </section>
          <section className="git-section">
            <div className="section-title">{t('git.commits')}</div>
            {overview.commits.map((c) => (
              <div className="commit-row" key={c.sha}>
                <span className="mono dim">{c.sha}</span>
                <span className="grow">{c.subject}</span>
                <span className="dim">
                  {c.author} · {relativeTime(c.date, t, locale)}
                </span>
              </div>
            ))}
          </section>
        </div>
      )}
    </div>
  )
}

function RepoHeader({ repo }: { repo: ProjectRepo }) {
  const { run, t } = useApp()
  const github = repo.github_owner ? `https://github.com/${repo.github_owner}/${repo.github_repo}` : null
  return (
    <div className="repo-header">
      <FolderGit2 size={18} />
      <span className="mono grow path-text" title={repo.path}>
        {repo.path}
      </span>
      {github && (
        <button className="btn ghost" onClick={() => run(api.openUrl(github))}>
          <ExternalLink size={13} /> {repo.github_owner}/{repo.github_repo}
        </button>
      )}
      <label className="toggle-label" title={t('git.autoMoveHint')}>
        <input type="checkbox" checked={repo.auto_move} onChange={(e) => run(api.setRepoAutoMove(repo.project_id, e.target.checked))} />
        {t('git.autoMove')}
      </label>
      <button className="btn ghost" onClick={() => run(api.unlinkRepo(repo.project_id))}>
        <Unlink size={13} /> {t('git.unlink')}
      </button>
    </div>
  )
}
