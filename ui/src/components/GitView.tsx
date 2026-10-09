// Git tab: link a repository, branches, recent commits and GitHub pull requests.
import { editorLabel, openInEditor } from '../editor'
import { useCallback, useEffect, useState, useSyncExternalStore } from 'react'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { CircleCheck, CircleDashed, Code2, CircleX, ExternalLink, FolderGit2, GitBranch, GitMerge, GitPullRequest, RefreshCw, Unlink, CornerUpLeft, CloudUpload, FolderPlus } from 'lucide-react'
import { api, errorMessage, relativeTime, remoteOf } from '../api'
import type { Checks, LinkedPullRequest, ProjectRepo } from '../api'
import { useApp, useLive } from '../data'
import type { MessageKey } from '../i18n'
import { NewRepoFields } from './NewRepoFields'
import { rememberParent } from '../repoName'
import type { RepoDraft } from './NewRepoFields'

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

/** Make a GitHub repository for a project that has none yet: new and cloned, or a local one published. */
function CreateRepo({ projectId, mode, onDone }: { projectId: number; mode: 'create' | 'publish'; onDone: () => void }) {
  const { run, t, toast } = useApp()
  const [projects] = useLive(() => api.listProjects(), [])
  const project = projects?.find((p) => p.id === projectId)
  const [draft, setDraft] = useState<RepoDraft | null>(null)
  const [busy, setBusy] = useState(false)
  const submit = async () => {
    if (!draft || busy) return
    setBusy(true)
    const action =
      mode === 'create' ? api.createGithubRepo(projectId, draft.repo, draft.folder) : api.publishToGithub(projectId, draft.repo, draft.folder)
    const linked = await run(action)
    setBusy(false)
    if (!linked) return
    if (mode === 'create') rememberParent(draft.folder)
    toast(t('newRepo.done', { repo: `${draft.repo.owner}/${draft.repo.name}` }), 'success')
    onDone()
  }
  if (!project) return null
  return (
    <div className="create-repo">
      <strong className="create-repo-title">{mode === 'create' ? t('newRepo.createOnGithub') : t('newRepo.publishFolder')}</strong>
      <NewRepoFields mode={mode} projectName={project.name} description={project.description ?? ''} onChange={setDraft} />
      <div className="row-inline">
        <button className="btn primary" disabled={!draft || busy} onClick={() => void submit()}>
          {busy ? <RefreshCw size={13} className="spin" /> : <CloudUpload size={14} />} {mode === 'create' ? t('newRepo.create') : t('newRepo.publish')}
        </button>
        <button className="btn ghost" disabled={busy} onClick={onDone}>
          {t('dialog.cancel')}
        </button>
      </div>
    </div>
  )
}

function LinkRepo({ projectId }: { projectId: number }) {
  const { run, t } = useApp()
  const [path, setPath] = useState('')
  const [linking, setLinking] = useState(false)
  const [creating, setCreating] = useState<'create' | 'publish' | null>(null)
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
            if (!path.trim() || linking) return
            setLinking(true)
            run(api.linkRepo(projectId, path.trim())).finally(() => setLinking(false))
          }}
        >
          <button type="button" className="btn" onClick={choose}>
            {t('git.choose')}
          </button>
          <input className="input mono grow" value={path} onChange={(e) => setPath(e.target.value)} placeholder={t('git.pathPlaceholder')} aria-label={t('git.pathPlaceholder')} />
          <button className="btn primary" disabled={!path.trim() || linking}>
            {linking ? (
              <>
                <RefreshCw size={13} className="spin" /> {t('git.linking')}
              </>
            ) : (
              t('git.link')
            )}
          </button>
        </form>
        {creating ? (
          <CreateRepo projectId={projectId} mode={creating} onDone={() => setCreating(null)} />
        ) : (
          <div className="row-inline create-repo-choices">
            <span className="hint">{t('newRepo.noRepoYet')}</span>
            <button className="btn ghost" onClick={() => setCreating('create')}>
              <FolderPlus size={14} /> {t('newRepo.createOnGithub')}
            </button>
            <button className="btn ghost" onClick={() => setCreating('publish')}>
              <CloudUpload size={14} /> {t('newRepo.publishFolder')}
            </button>
          </div>
        )}
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
  const hasGithub = !!remoteOf(repo)

  const sync = useCallback(
    async (quiet = false) => {
      setSyncing(true)
      setSyncError(null)
      try {
        const report = await refreshPullRequests(projectId)
        if (report.completed.length > 0) toast(t('git.completed', { n: report.completed.length }), 'success')
        else if (!quiet) toast(t('git.synced', { n: report.pull_requests.length }), 'success')
      } catch (err) {
        setSyncError(`${t('git.syncFailed')}: ${errorMessage(err)}`)
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
      {overview === undefined && (
        <div className="loading-line">
          <RefreshCw size={13} className="spin" /> {t('git.loading')}
        </div>
      )}
      {overview && (
        <div className="git-status">
          <GitBranch size={14} /> {t('git.onBranch', { branch: overview.current_branch || 'HEAD' })}
          {overview.dirty && <span className="chip warn-chip">{t('git.dirty')}</span>}
          <button className="btn ghost" onClick={() => run(openInEditor(projectId)).then((p) => p && toast(t('editor.opened', { editor: editorLabel() }), 'success'))}>
            <Code2 size={13} /> {t('editor.open')}
          </button>
          {overview.default_branch && overview.current_branch !== overview.default_branch && (
            <button className="btn ghost" onClick={() => run(api.checkoutDefault(projectId)).then((b) => b && toast(t('git.switchedTo', { branch: b }), 'success'))}>
              <CornerUpLeft size={13} /> {t('git.backTo', { branch: overview.default_branch })}
            </button>
          )}
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
              {(pr.task_ids ?? (pr.task_id ? [pr.task_id] : [])).map((id) => (
                <button key={id} className="chip" onClick={() => onOpenTask(id)}>
                  {t('git.task', { id })}
                </button>
              ))}
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
  const remote = remoteOf(repo)
  return (
    <div className="repo-header">
      <FolderGit2 size={18} />
      <span className="mono grow path-text" title={repo.path}>
        {repo.path}
      </span>
      {remote && (
        <button className="btn ghost" onClick={() => run(api.openUrl(remote.url))} title={remote.url}>
          <ExternalLink size={13} /> {remote.label}
          {remote.kind === 'azure' && <span className="beta">Azure DevOps</span>}
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
