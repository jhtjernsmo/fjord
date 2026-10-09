// Slide-in task details: title, status, priority, due date, markdown, files.
import { editorLabel, openInEditor } from '../editor'
import { useEffect, useState } from 'react'
import Markdown from 'react-markdown'
import { MARKDOWN_COMPONENTS, MARKDOWN_PLUGINS } from './NoteMarkdown'
import { motion } from 'motion/react'
import { Archive, CircleCheck, Code2, ExternalLink, GitBranch, GitPullRequest, Link2, Trash2, Unlink, X, CornerUpLeft, CornerDownRight } from 'lucide-react'
import { useContextMenus, useEntityActions } from './actions'
import { api, formatBytes, relativeTime, remoteOf } from '../api'
import type { Attachment, Status, Task, TaskPatch } from '../api'
import { useActions, useApp, useLive } from '../data'
import type { MessageKey } from '../i18n'
import { AgentTag, FileTypeIcon } from './Icons'
import { ChecksIcon, PrStateBadge, refreshPullRequests, usePullRequests } from './GitView'
import { Backlinks } from './NoteEditor'
import { copyTaskRef } from './taskRef'
import { Subtasks } from './Subtasks'
import { AzureDiscussion } from './AzureDiscussion'

const PRIORITY_LEVELS = [0, 1, 2, 3]

export function FileTile({ file, onDetach }: { file: Attachment; onDetach?: () => void }) {
  const { run, t } = useApp()
  const { fileMenu } = useContextMenus()
  const [preview, setPreview] = useState<string | null>(null)
  useEffect(() => {
    let cancelled = false
    api
      .previewAttachment(file.id)
      .then((url) => !cancelled && setPreview(url))
      .catch(() => undefined)
    return () => {
      cancelled = true
    }
  }, [file.id])
  return (
    <div className="file" onContextMenu={fileMenu(file)}>
      <button className="thumb" onClick={() => run(api.openAttachment(file.id))} title={t('file.open')}>
        {preview ? <img src={preview} alt={file.original_name} /> : <FileTypeIcon name={file.original_name} />}
      </button>
      <div className="meta">
        <div className="fname" title={file.original_name}>
          {file.original_name}
        </div>
        <div className="sub">
          <span>
            {formatBytes(file.size)} · {file.added_by === 'claude' ? <AgentTag /> : file.added_by}
          </span>
          {onDetach && (
            <button onClick={onDetach} title={t('file.remove')} aria-label={t('file.remove')}>
              <X size={12} />
            </button>
          )}
        </div>
      </div>
    </div>
  )
}

interface Props {
  taskId: number
  statuses: Status[]
  onClose: () => void
}

export function TaskPanel({ taskId, statuses, onClose }: Props) {
  const { run, t, locale, toast } = useApp()
  const { deleteTask } = useEntityActions()
  const projectId = statuses[0].project_id
  // undefined while loading, null if the task disappeared (e.g. archived elsewhere).
  const [task] = useLive(async () => (await api.getBoard(projectId)).tasks.find((x) => x.id === taskId) ?? null, [taskId])
  const [files] = useLive(() => api.listAttachments(projectId, taskId), [taskId])
  const [title, setTitle] = useState('')
  const [body, setBody] = useState('')
  const [editing, setEditing] = useState(false)

  useEffect(() => {
    if (!task) return
    setTitle(task.title)
    if (!editing) setBody(task.body_md)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [task?.id, task?.title, task?.body_md])

  useEffect(() => {
    if (task === null) onClose()
  }, [task, onClose])

  // Keys while a task is open (the board's keys are off then).
  const moveBy = (delta: number) => {
    if (!task) return
    const i = statuses.findIndex((s) => s.id === task.status_id)
    const target = statuses[i + delta]
    if (target) run(api.moveTask(task.id, target.id))
  }
  useActions(
    task
      ? {
          'panel.prevColumn': () => moveBy(-1),
          'panel.nextColumn': () => moveBy(1),
          'panel.priority': () => run(api.updateTask(task.id, { priority: (task.priority + 1) % 4 })),
          'panel.edit': () => setEditing(true),
          'panel.copyRef': () => copyTaskRef(task, toast, t),
          'panel.archive': () => run(api.archiveTask(task.id, true), t('toast.archivedTask', { name: task.title })),
        }
      : {},
    'panel',
  )

  if (!task) return null

  const save = (patch: TaskPatch) => run(api.updateTask(task.id, patch))
  const status = statuses.find((s) => s.id === task.status_id)
  const creator = task.created_by === 'claude' ? <AgentTag /> : task.created_by

  return (
    <>
      <div className="scrim" onClick={onClose} />
      <motion.aside
        className="panel"
        initial={{ x: 60, opacity: 0 }}
        animate={{ x: 0, opacity: 1 }}
        transition={{ type: 'spring', stiffness: 420, damping: 36 }}
        aria-label={task.title}
      >
        <div className="panel-head">
          <span className="mono">#{task.id}</span>
          <span>
            {creator} · {relativeTime(task.created_at, t, locale)}
          </span>
          <span className="spacer" />
          <button className="btn ghost" onClick={() => run(api.archiveTask(task.id, true), t('task.archived')).then(onClose)}>
            <Archive size={14} /> {t('task.archive')}
          </button>
          <button className="btn danger-ghost" onClick={() => deleteTask(task, onClose)} title={t('menu.delete')} aria-label={t('menu.delete')}>
            <Trash2 size={14} />
          </button>
          <button className="btn ghost" onClick={onClose} aria-label={t('task.close')}>
            <X size={14} /> <kbd>Esc</kbd>
          </button>
        </div>
        <div className="panel-body">
          <input
            className="title-input"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            onBlur={() => title.trim() && title !== task.title && save({ title })}
            onKeyDown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
            aria-label={t('task.title')}
          />

          <div className="fields">
            <label>{t('task.status')}</label>
            <select className="input" value={task.status_id} onChange={(e) => run(api.moveTask(task.id, Number(e.target.value)))}>
              {statuses.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                </option>
              ))}
            </select>

            <label>{t('task.priority')}</label>
            <div className="segmented" role="radiogroup" aria-label={t('task.priority')}>
              {PRIORITY_LEVELS.map((level) => (
                <button
                  key={level}
                  className={task.priority === level ? 'on' : ''}
                  onClick={() => save({ priority: level })}
                  role="radio"
                  aria-checked={task.priority === level}
                >
                  {t(`priority.${level}` as MessageKey)}
                </button>
              ))}
            </div>

            <label>{t('task.due')}</label>
            <div className="row-inline">
              <input type="date" className="input" value={task.due_at ?? ''} onChange={(e) => save({ due_at: e.target.value || null })} />
              {task.due_at && (
                <button className="btn ghost" onClick={() => save({ due_at: null })}>
                  {t('task.clear')}
                </button>
              )}
            </div>
          </div>

          <div>
            <div className="section-title">
              {t('task.description')}
              <button onClick={() => (editing ? (save({ body_md: body }), setEditing(false)) : setEditing(true))}>
                {editing ? t('task.save') : t('task.edit')}
              </button>
            </div>
            {editing ? (
              <textarea
                className="md-editor"
                autoFocus
                value={body}
                onChange={(e) => setBody(e.target.value)}
                onBlur={() => {
                  if (body !== task.body_md) save({ body_md: body })
                  setEditing(false)
                }}
                onKeyDown={(e) => e.key === 'Escape' && (e.stopPropagation(), e.currentTarget.blur())}
                placeholder={t('task.descriptionPlaceholder')}
              />
            ) : (
              <div className={`markdown ${task.body_md ? '' : 'empty'}`} onClick={() => setEditing(true)}>
                {task.body_md ? <Markdown remarkPlugins={MARKDOWN_PLUGINS} components={MARKDOWN_COMPONENTS}>{task.body_md}</Markdown> : t('task.addDescription')}
              </div>
            )}
          </div>

          <div>
            <div className="section-title">{t('task.files', { n: files?.length ?? 0 })}</div>
            <div className="files" style={{ marginTop: 10 }}>
              {(files ?? []).map((f) => (
                <FileTile key={f.id} file={f} onDetach={() => run(api.detachFile(f.id))} />
              ))}
            </div>
            <div className="dropzone" style={{ marginTop: 10 }}>
              {t('task.dropHere', { title: task.title })}
            </div>
          </div>

          <Subtasks task={task} statuses={statuses} />

          <AzureDiscussion taskId={task.id} />

          <GitSection task={task} />

          <Backlinks kind="task" id={task.id} />

          {status?.is_done && (
            <div className="chip done-chip">
              <CircleCheck size={13} /> {t('task.done')}
            </div>
          )}
        </div>
      </motion.aside>
    </>
  )
}

const BRANCH_TYPES = ['feat', 'fix', 'chore', 'docs', 'refactor', 'test', 'perf', 'ci', 'hotfix']

/** Start a conventional branch: pick the type, see the name, go. */
function StartBranch({ task, kind, onKind }: { task: Task; kind: string | null; onKind: (k: string) => void }) {
  const { run, t, toast } = useApp()
  const [suggestion] = useLive(() => api.suggestBranch(task.id, kind), [task.id, task.title, kind])
  const current = kind ?? suggestion?.[0] ?? 'feat'
  return (
    <div className="start-branch">
      <select className="input mono" value={current} onChange={(e) => onKind(e.target.value)} aria-label={t('git.branchType')} title={t('git.branchType')}>
        {BRANCH_TYPES.map((k) => (
          <option key={k} value={k}>
            {k}
          </option>
        ))}
      </select>
      <span className="mono dim branch-preview" title={suggestion?.[1]}>
        {suggestion?.[1] ?? ''}
      </span>
      <button className="btn" onClick={() => run(api.startBranch(task.id, current)).then((r) => r && toast(t('git.branchStarted', { branch: r.branch }), 'success'))}>
        <GitBranch size={14} /> {t('git.startBranch')} <kbd>B</kbd>
      </button>
    </div>
  )
}

/** Pick an existing branch (local or on origin) to link to the task. */
function LinkBranch({ task, onDone }: { task: Task; onDone: () => void }) {
  const { run, t, toast } = useApp()
  const [branches] = useLive(() => api.listBranches(task.project_id), [task.project_id])
  const [query, setQuery] = useState('')
  const [index, setIndex] = useState(0)
  const needle = query.trim().toLowerCase()
  const matches = (branches ?? []).filter((b) => b.toLowerCase().includes(needle)).slice(0, 8)
  const link = async (branch: string) => {
    const linked = await run(api.linkTaskBranch(task.id, branch))
    if (linked) {
      toast(t('git.branchLinked', { branch }), 'success')
      refreshPullRequests(task.project_id).catch(() => undefined)
      onDone()
    }
  }
  return (
    <div className="branch-picker">
      <input
        className="input mono"
        autoFocus
        value={query}
        placeholder={t('git.findBranch')}
        aria-label={t('git.findBranch')}
        onChange={(e) => {
          setQuery(e.target.value)
          setIndex(0)
        }}
        onKeyDown={(e) => {
          if (e.key === 'ArrowDown') setIndex((i) => Math.min(i + 1, matches.length - 1))
          else if (e.key === 'ArrowUp') setIndex((i) => Math.max(i - 1, 0))
          else if (e.key === 'Enter' && matches[index]) link(matches[index])
          else if (e.key === 'Escape') onDone()
          else return
          e.preventDefault()
          e.stopPropagation()
        }}
      />
      <div className="branch-list" role="listbox">
        {branches === undefined && <span className="hint">{t('github.checking')}</span>}
        {branches && matches.length === 0 && <span className="hint">{t('git.noBranches')}</span>}
        {matches.map((b, i) => (
          <button key={b} role="option" aria-selected={i === index} className={i === index ? 'on' : ''} onMouseEnter={() => setIndex(i)} onClick={() => link(b)}>
            <GitBranch size={13} /> <span className="mono">{b}</span>
          </button>
        ))}
      </div>
      <button className="btn ghost" onClick={onDone}>
        {t('dialog.cancel')}
      </button>
    </div>
  )
}

function GitSection({ task }: { task: Task }) {
  const { run, t, toast } = useApp()
  const [repo] = useLive(() => api.getProjectRepo(task.project_id), [task.project_id])
  const prs = usePullRequests(task.project_id)
  const [busy, setBusy] = useState(false)
  const [picking, setPicking] = useState(false)
  const [kind, setKind] = useState<string | null>(null)
  const pr = prs?.find((p) => (p.task_ids ?? [p.task_id]).includes(task.id))
  const hasGithub = !!remoteOf(repo)
  // Fetch PRs once if nothing has synced this project yet (e.g. Git tab never opened).
  useEffect(() => {
    if (hasGithub && prs === undefined) refreshPullRequests(task.project_id).catch(() => undefined)
  }, [hasGithub, prs, task.project_id])

  const openPr = async (draft: boolean) => {
    setBusy(true)
    const created = await run(api.openPullRequest(task.id, draft))
    if (created) {
      toast(t('git.prOpened', { n: created.number }), 'success')
      refreshPullRequests(task.project_id).catch(() => undefined)
    }
    setBusy(false)
  }

  useActions(
    repo
      ? {
          'panel.branch': () =>
            run(api.startBranch(task.id, kind)).then((r) => r && toast(t('git.branchStarted', { branch: r.branch }), 'success')),
          'panel.linkBranch': () => !task.branch && setPicking(true),
          'panel.openPr': () => task.branch && hasGithub && !pr && !busy && openPr(false),
        }
      : {},
    'panel',
  )
  if (repo === undefined) return null

  return (
    <div>
      <div className="section-title">Git</div>
      {repo === null ? (
        <div className="hint" style={{ marginTop: 6 }}>
          {t('git.notLinked')}
        </div>
      ) : (
        <div className="task-git">
          {task.branch ? (
            <div className="linked-branch">
              <GitBranch size={14} />
              <span className="mono branch-name" title={task.branch}>
                {task.branch}
              </span>
              <button className="icon-btn" onClick={() => run(api.startBranch(task.id), t('git.branchStarted', { branch: task.branch ?? '' }))} title={t('git.checkoutHint')} aria-label={t('git.checkout')}>
                <CornerDownRight size={14} />
              </button>
              <button className="icon-btn" onClick={() => run(openInEditor(task.project_id, task.id)).then((p) => p && toast(t('editor.opened', { editor: editorLabel() }), 'success'))} title={t('editor.openTaskHint')} aria-label={t('editor.open')}>
                <Code2 size={14} />
              </button>
              <button className="icon-btn" onClick={() => run(api.checkoutDefault(task.project_id)).then((b) => b && toast(t('git.switchedTo', { branch: b }), 'success'))} title={t('git.backToMainHint')} aria-label={t('git.backToMain')}>
                <CornerUpLeft size={14} />
              </button>
              <button className="icon-btn" onClick={() => run(api.linkTaskBranch(task.id, null))} title={t('git.unlinkBranch')} aria-label={t('git.unlinkBranch')}>
                <Unlink size={14} />
              </button>
            </div>
          ) : picking ? (
            <LinkBranch task={task} onDone={() => setPicking(false)} />
          ) : (
            <div className="branch-actions">
              <StartBranch task={task} kind={kind} onKind={setKind} />
              <button className="btn ghost" onClick={() => setPicking(true)}>
                <Link2 size={14} /> {t('git.linkBranch')}
              </button>
            </div>
          )}
          {pr ? (
            <div className="pr-row">
              <ChecksIcon checks={pr.checks} />
              <span className="mono dim">#{pr.number}</span>
              <span className="pr-title">{pr.title}</span>
              <PrStateBadge pr={pr} />
              <button className="icon-btn" onClick={() => run(api.openUrl(pr.url))} aria-label={t('git.openOnGithub')}>
                <ExternalLink size={14} />
              </button>
            </div>
          ) : (
            task.branch &&
            hasGithub && (
              <div className="row-inline">
                <button className="btn primary" disabled={busy} onClick={() => openPr(false)}>
                  <GitPullRequest size={14} /> {t('git.openPr')}
                </button>
                <button className="btn ghost" disabled={busy} onClick={() => openPr(true)}>
                  {t('git.openDraft')}
                </button>
              </div>
            )
          )}
        </div>
      )}
    </div>
  )
}
