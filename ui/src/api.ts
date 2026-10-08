// Typed wrappers around the Tauri commands in src-tauri/src/commands.rs.
import { invoke } from '@tauri-apps/api/core'
import { dateLocale } from './i18n'
import type { Locale, MessageKey, Translate } from './i18n'

export interface Project {
  id: number
  name: string
  slug: string
  description: string
  color: string
  icon: string
  created_at: string
  updated_at: string
  archived_at: string | null
}

export interface ProjectSummary extends Project {
  task_count: number
  done_count: number
  overdue_count: number
}

export interface Status {
  id: number
  project_id: number
  name: string
  color: string
  position: number
  is_done: boolean
}

export interface Task {
  id: number
  project_id: number
  status_id: number
  title: string
  body_md: string
  priority: number
  due_at: string | null
  position: number
  created_by: string
  created_at: string
  updated_at: string
  archived_at: string | null
  branch: string | null
}

export interface Board {
  project: Project
  statuses: Status[]
  tasks: Task[]
}

export interface Attachment {
  id: number
  project_id: number
  task_id: number | null
  original_name: string
  sha256: string
  size: number
  added_by: string
  created_at: string
}

export interface AttachOutcome {
  path: string
  attachment: Attachment | null
  error: string | null
}

export interface Note {
  id: number
  project_id: number | null
  title: string
  body_md: string
  folder: string
  pinned: boolean
  created_at: string
  updated_at: string
}

export interface GitHubAccount {
  login: string
  source: 'env' | 'saved' | 'gh_cli'
  private_repos: boolean
}

export interface ImportMapping {
  org: string
  project: string
  fjord_project_id: number
}

export interface ImportSettings {
  enabled: boolean
  mappings: ImportMapping[]
}

export interface ImportReport {
  created: Task[]
  updated: number
  unchanged: number
  unmapped: number
}

export interface DeviceLogin {
  user_code: string
  verification_uri: string
  interval: number
  expires_in: number
}

export type DevicePoll =
  | { status: 'pending' }
  | { status: 'slow_down'; interval: number }
  | { status: 'done'; account: GitHubAccount }
  | { status: 'expired' }
  | { status: 'denied' }

export interface AzureAccount {
  org: string
  user: string
  source: 'env' | 'saved' | 'cli'
}

/** Where a linked repository's pull requests live, for links and labels. */
export function remoteOf(repo: ProjectRepo | null | undefined): { kind: 'github' | 'azure'; label: string; url: string } | null {
  if (!repo) return null
  if (repo.github_owner && repo.github_repo) {
    return { kind: 'github', label: `${repo.github_owner}/${repo.github_repo}`, url: `https://github.com/${repo.github_owner}/${repo.github_repo}` }
  }
  if (repo.azure_org && repo.azure_project && repo.azure_repo) {
    const enc = encodeURIComponent
    return {
      kind: 'azure',
      label: `${repo.azure_project}/${repo.azure_repo}`,
      url: `https://dev.azure.com/${enc(repo.azure_org)}/${enc(repo.azure_project)}/_git/${enc(repo.azure_repo)}`,
    }
  }
  return null
}

export interface LinkTarget {
  kind: 'project' | 'task' | 'note'
  id: number
  label: string
  project_id: number | null
  done: boolean
}

export interface Activity {
  id: number
  project_id: number | null
  task_id: number | null
  actor: string
  action: string
  subject: string
  detail: string | null
  created_at: string
}

export interface SearchHit {
  kind: 'task' | 'note' | 'file'
  ref_id: number
  project_id: number | null
  title: string
  snippet: string
}

export interface ProjectRepo {
  project_id: number
  path: string
  github_owner: string | null
  github_repo: string | null
  azure_org: string | null
  azure_project: string | null
  azure_repo: string | null
  auto_move: boolean
}

export interface GitBranch {
  name: string
  sha: string
  current: boolean
  upstream: string | null
  ahead: number
  behind: number
}

export interface GitCommit {
  sha: string
  author: string
  date: string
  subject: string
}

export interface GitOverview {
  repo: ProjectRepo
  current_branch: string
  dirty: boolean
  branches: GitBranch[]
  commits: GitCommit[]
}

export type Checks = 'none' | 'pending' | 'success' | 'failure'

export interface PullRequest {
  number: number
  title: string
  state: 'open' | 'closed' | 'merged'
  draft: boolean
  url: string
  author: string
  head: string
  base: string
  head_sha: string
  updated_at: string
  checks: Checks
}

export interface LinkedPullRequest extends PullRequest {
  task_id: number | null
}

export interface SyncReport {
  pull_requests: LinkedPullRequest[]
  completed: Task[]
}

export interface StartedBranch {
  task: Task
  branch: string
  created: boolean
}

export interface NewProject {
  name: string
  locale?: string
  description?: string
  color?: string
  icon?: string
}

export interface TaskPatch {
  title?: string
  body_md?: string
  priority?: number
  /** `null` clears the due date; omit to keep it. */
  due_at?: string | null
}

export const api = {
  actor: () => invoke<string>('actor'),
  changeCounter: () => invoke<number>('change_counter'),
  listProjects: (includeArchived = false) => invoke<ProjectSummary[]>('list_projects', { includeArchived }),
  createProject: (input: NewProject) => invoke<Project>('create_project', { input }),
  updateProject: (id: number, patch: Partial<NewProject>) => invoke<Project>('update_project', { id, patch }),
  archiveProject: (id: number, archived: boolean) => invoke<Project>('archive_project', { id, archived }),
  getBoard: (projectId: number) => invoke<Board>('get_board', { projectId }),
  createTask: (input: { project_id: number; title: string; status_id?: number; priority?: number; due_at?: string }) =>
    invoke<Task>('create_task', { input }),
  updateTask: (id: number, patch: TaskPatch) => invoke<Task>('update_task', { id, patch }),
  moveTask: (id: number, statusId: number, beforeTaskId: number | null = null) =>
    invoke<Task>('move_task', { id, statusId, beforeTaskId }),
  archiveTask: (id: number, archived: boolean) => invoke<Task>('archive_task', { id, archived }),
  deleteTask: (id: number) => invoke<void>('delete_task', { id }),
  deleteProject: (id: number) => invoke<void>('delete_project', { id }),
  deleteNote: (id: number) => invoke<void>('delete_note', { id }),
  renameUser: (name: string, rewriteHistory: boolean) => invoke<string>('rename_user', { name, rewriteHistory }),
  listArchivedTasks: (projectId: number) => invoke<Task[]>('list_archived_tasks', { projectId }),
  createStatus: (projectId: number, name: string, color: string | null, isDone: boolean) =>
    invoke<Status>('create_status', { projectId, name, color, isDone }),
  updateStatus: (id: number, patch: { name?: string; color?: string; is_done?: boolean }) =>
    invoke<Status>('update_status', { id, patch }),
  moveStatus: (id: number, index: number) => invoke<Status[]>('move_status', { id, index }),
  deleteStatus: (id: number) => invoke<void>('delete_status', { id }),
  listAttachments: (projectId: number, taskId: number | null = null) =>
    invoke<Attachment[]>('list_attachments', { projectId, taskId }),
  attachFiles: (projectId: number, taskId: number | null, paths: string[]) =>
    invoke<AttachOutcome[]>('attach_files', { projectId, taskId, paths }),
  detachFile: (id: number) => invoke<void>('detach_file', { id }),
  openAttachment: (id: number) => invoke<void>('open_attachment', { id }),
  previewAttachment: (id: number) => invoke<string | null>('preview_attachment', { id }),
  listNotes: (projectId: number) => invoke<Note[]>('list_notes', { projectId }),
  addNote: (projectId: number, title: string, bodyMd: string) => invoke<Note>('add_note', { projectId, title, bodyMd }),
  updateNote: (id: number, title: string, bodyMd: string) => invoke<Note>('update_note', { id, title, bodyMd }),
  search: (query: string) => invoke<SearchHit[]>('search', { query }),
  listAllNotes: (freeOnly = false) => invoke<Note[]>('list_all_notes', { freeOnly }),
  createNote: (projectId: number | null, title: string, bodyMd: string) => invoke<Note>('create_note', { projectId, title, bodyMd }),
  getNote: (id: number) => invoke<Note>('get_note', { id }),
  moveNote: (id: number, projectId: number | null) => invoke<Note>('move_note', { id, projectId }),
  setNoteFolder: (id: number, folder: string) => invoke<Note>('set_note_folder', { id, folder }),
  setNotePinned: (id: number, pinned: boolean) => invoke<Note>('set_note_pinned', { id, pinned }),
  resolveLinks: (texts: string[]) => invoke<(LinkTarget | null)[]>('resolve_links', { texts }),
  backlinks: (kind: LinkTarget['kind'], targetId: number) => invoke<Note[]>('backlinks', { kind, targetId }),
  linkSuggestions: (query: string) => invoke<LinkTarget[]>('link_suggestions', { query }),
  recentActivity: (projectId: number | null, limit = 30) => invoke<Activity[]>('recent_activity', { projectId, limit }),
  loadKeymap: () => invoke<unknown>('load_keymap'),
  dataPaths: () => invoke<{ data: string; keymap: string }>('data_paths'),
  getProjectRepo: (projectId: number) => invoke<ProjectRepo | null>('get_project_repo', { projectId }),
  linkRepo: (projectId: number, path: string) => invoke<ProjectRepo>('link_repo', { projectId, path }),
  unlinkRepo: (projectId: number) => invoke<void>('unlink_repo', { projectId }),
  setRepoAutoMove: (projectId: number, autoMove: boolean) => invoke<void>('set_repo_auto_move', { projectId, autoMove }),
  gitOverview: (projectId: number) => invoke<GitOverview>('git_overview', { projectId }),
  startBranch: (taskId: number) => invoke<StartedBranch>('start_branch', { taskId }),
  syncPullRequests: (projectId: number) => invoke<SyncReport>('sync_pull_requests', { projectId }),
  openPullRequest: (taskId: number, draft: boolean) => invoke<PullRequest>('open_pull_request', { taskId, draft }),
  openUrl: (url: string) => invoke<void>('open_url', { url }),
  githubAccount: () => invoke<GitHubAccount | null>('github_account'),
  connectGithub: (token: string) => invoke<GitHubAccount>('connect_github', { token }),
  disconnectGithub: () => invoke<void>('disconnect_github'),
  startGithubLogin: () => invoke<DeviceLogin>('start_github_login'),
  pollGithubLogin: () => invoke<DevicePoll>('poll_github_login'),
  connectAzureCli: (org: string) => invoke<AzureAccount>('connect_azure_cli', { org }),
  azureAccount: (org: string) => invoke<AzureAccount | null>('azure_account', { org }),
  connectAzure: (org: string, token: string) => invoke<AzureAccount>('connect_azure', { org, token }),
  disconnectAzure: (org: string) => invoke<void>('disconnect_azure', { org }),
  getImportSettings: () => invoke<ImportSettings>('get_import_settings'),
  setImportSettings: (settings: ImportSettings) => invoke<void>('set_import_settings', { settings }),
  runAzureImport: () => invoke<ImportReport>('run_azure_import'),
}

export function errorMessage(err: unknown): string {
  if (typeof err === 'string') return err
  if (err instanceof Error) return err.message
  return JSON.stringify(err)
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

export function relativeTime(iso: string, t: Translate, locale: Locale): string {
  const seconds = (Date.now() - new Date(iso).getTime()) / 1000
  if (seconds < 60) return t('time.now')
  if (seconds < 3600) return t('time.min', { n: Math.floor(seconds / 60) })
  if (seconds < 86400) return t('time.hour', { n: Math.floor(seconds / 3600) })
  return new Date(iso).toLocaleDateString(dateLocale(locale), { day: 'numeric', month: 'short' })
}

export function describeActivity(a: Activity, t: Translate): string {
  const key = `act.${a.action}` as MessageKey
  return t(key, { s: a.subject, d: a.detail ?? '' })
}

export function todayIso(): string {
  return new Date().toISOString().slice(0, 10)
}

export function isOverdue(task: Task, statuses: Status[]): boolean {
  if (!task.due_at) return false
  const done = statuses.find((s) => s.id === task.status_id)?.is_done
  return !done && task.due_at < todayIso()
}
