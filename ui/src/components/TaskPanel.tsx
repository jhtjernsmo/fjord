// Slide-in task details: title, status, priority, due date, markdown, files.
import { editorLabel, openInEditor } from '../editor'
import { useEffect, useMemo, useRef, useState } from 'react'
import Markdown from 'react-markdown'
import { MARKDOWN_COMPONENTS, MARKDOWN_PLUGINS } from './NoteMarkdown'
import { motion } from 'motion/react'
import { Archive, ArrowRightLeft, CircleCheck, Hourglass, PictureInPicture2, ClipboardList, Code2, ExternalLink, GitBranch, GitPullRequest, Link2, Trash2, Unlink, X, CornerUpLeft, CornerDownRight } from 'lucide-react'
import { useContextMenus, useEntityActions, useStoryAnalysis } from './actions'
import { api, formatBytes, relativeTime, remoteOf } from '../api'
import type { Attachment, Status, Task, TaskPatch } from '../api'
import { useActions, useApp, useLive } from '../data'
import { dateLocale } from '../i18n'
import type { MessageKey } from '../i18n'
import { waitedFor } from '../pending'
import { azureAttachmentId, localCopies } from '../azureImages'
import { AgentTag, FileTypeIcon } from './Icons'
import { ChecksIcon, PrStateBadge, refreshPullRequests, usePullRequests } from './GitView'
import { Backlinks } from './NoteEditor'
import { copyTaskRef } from './taskRef'
import { Subtasks } from './Subtasks'
import { AzureDiscussion } from './AzureDiscussion'
import { useStoredPref } from '../storedPref'

const PRIORITY_LEVELS = [0, 1, 2, 3]
/** Narrowest the panel gets: wide enough for every button in its header. */
const PANEL_MIN = 480
const PANEL_DEFAULT = 520
/** Room left of the panel when dragged as wide as it goes. */
const PANEL_MARGIN = 160
const parseWidth = (raw: unknown) => (typeof raw === 'number' && raw > 0 ? raw : PANEL_DEFAULT)

/** The panel's width, dragged by its left edge and remembered on this device. */
function usePanelWidth() {
  const [saved, save] = useStoredPref('fjord.taskPanelWidth', parseWidth, PANEL_DEFAULT)
  const [dragging, setDragging] = useState<number | null>(null)
  const ref = useRef<HTMLElement>(null)
  const clamp = (w: number) => Math.round(Math.min(Math.max(w, PANEL_MIN), window.innerWidth - PANEL_MARGIN))
  const handle = {
    onPointerDown: (e: React.PointerEvent) => {
      e.currentTarget.setPointerCapture(e.pointerId)
      setDragging(clamp(saved))
    },
    onPointerMove: (e: React.PointerEvent) => {
      const right = ref.current?.getBoundingClientRect().right
      if (dragging !== null && right !== undefined) setDragging(clamp(right - e.clientX))
    },
    onPointerUp: () => {
      if (dragging !== null) save(() => dragging)
      setDragging(null)
    },
    onDoubleClick: () => save(() => PANEL_DEFAULT),
  }
  const width = Math.max(dragging ?? saved, PANEL_MIN)
  return { ref, handle, style: { width: `min(${width}px, 100%)` }, dragging: dragging !== null }
}

/** An image in the description; Azure images show the copy the import downloaded. */
function DescriptionImage({ src, alt, local }: { src?: string; alt?: string; local: Map<string, number> }) {
  const { t } = useApp()
  const id = azureAttachmentId(src)
  const attachment = id ? local.get(id) : undefined
  const [url, setUrl] = useState<string | null>(null)
  useEffect(() => {
    if (attachment === undefined) return
    let cancelled = false
    api
      .previewAttachment(attachment)
      .then((u) => !cancelled && setUrl(u))
      .catch(() => undefined)
    return () => {
      cancelled = true
    }
  }, [attachment])
  if (!id) return <img className="md-image" src={src} alt={alt} />
  if (url === null) return <span className="md-image-missing">{alt || t('task.imageNotDownloaded')}</span>
  return <img className="md-image" src={url} alt={alt} />
}

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
  /** Called when the task is no longer on the board (archived, deleted, moved); defaults to onClose. */
  onGone?: () => void
  /** Shown in its own window: fills it, with no pop-out or resize handle. */
  detached?: boolean
}

export function TaskPanel({ taskId, statuses, onClose, onGone = onClose, detached = false }: Props) {
  const { run, t, locale, toast } = useApp()
  const { deleteTask, moveToProject, togglePending } = useEntityActions()
  const analyze = useStoryAnalysis()
  const projectId = statuses[0].project_id
  // undefined while loading, null if the task disappeared (e.g. archived elsewhere).
  const [task] = useLive(async () => (await api.getBoard(projectId)).tasks.find((x) => x.id === taskId) ?? null, [taskId])
  const [files] = useLive(() => api.listAttachments(projectId, taskId), [taskId])
  const [analysis] = useLive(() => api.storyAnalysis(taskId).catch(() => null), [taskId])
  const [externalFiles] = useLive(() => api.externalFiles(taskId).catch(() => []), [taskId])
  const descriptionComponents = useMemo(() => {
    const local = localCopies(externalFiles ?? [])
    return { ...MARKDOWN_COMPONENTS, img: ({ src, alt }: { src?: string; alt?: string }) => <DescriptionImage src={src} alt={alt} local={local} /> }
  }, [externalFiles])
  const panel = usePanelWidth()
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
    if (task === null) onGone()
  }, [task, onGone])

  const popOut = (t: Task) => {
    if (detached) return
    run(api.openTaskWindow(t.id, t.title)).then(onClose)
  }

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
          'panel.analyze': () => analyze(task),
          'panel.moveProject': () => moveToProject(task, detached ? undefined : onClose),
          'panel.popOut': () => popOut(task),
          'panel.pending': () => togglePending(task),
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
      {!detached && <div className="scrim" onClick={onClose} />}
      <motion.aside
        ref={panel.ref}
        className={`panel${panel.dragging ? ' resizing' : ''}${detached ? ' detached' : ''}`}
        style={detached ? undefined : panel.style}
        initial={detached ? false : { x: 60, opacity: 0 }}
        animate={{ x: 0, opacity: 1 }}
        transition={{ type: 'spring', stiffness: 420, damping: 36 }}
        aria-label={task.title}
      >
        {!detached && <div className="panel-resize" role="separator" aria-orientation="vertical" aria-label={t('task.resize')} title={t('task.resize')} {...panel.handle} />}
        <div className="panel-head">
          <span className="mono">#{task.id}</span>
          <span className="panel-meta">
            {creator} · {relativeTime(task.created_at, t, locale)}
          </span>
          <span className="spacer" />
          <button className="btn ghost" onClick={() => analyze(task)} title={t('analysis.hint')}>
            <ClipboardList size={14} /> {analysis ? t('analysis.open') : t('analysis.short')}
          </button>
          <button className="btn ghost" onClick={() => moveToProject(task, detached ? undefined : onClose)} title={t('moveProject.title')} aria-label={t('moveProject.title')}>
            <ArrowRightLeft size={14} />
          </button>
          {!detached && (
            <button className="btn ghost" onClick={() => popOut(task)} title={t('task.popOut')} aria-label={t('task.popOut')}>
              <PictureInPicture2 size={14} />
            </button>
          )}
          <button className="btn ghost" onClick={() => run(api.archiveTask(task.id, true), t('task.archived')).then(onClose)} title={t('task.archive')} aria-label={t('task.archive')}>
            <Archive size={14} />
          </button>
          <button className="btn danger-ghost" onClick={() => deleteTask(task, onClose)} title={t('menu.delete')} aria-label={t('menu.delete')}>
            <Trash2 size={14} />
          </button>
          <button className="btn ghost" onClick={onClose} aria-label={t('task.close')}>
            <X size={14} /> {!detached && <kbd>Esc</kbd>}
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

            <label>{t('pending.title')}</label>
            <div className="row-inline pending-field">
              {task.pending_since ? (
                <>
                  <span className="pending-since">
                    <Hourglass size={13} />{' '}
                    {t('pending.since', { date: new Date(task.pending_since).toLocaleDateString(dateLocale(locale), { day: 'numeric', month: 'long' }) })}
                    {' · '}
                    {waitedFor(task.pending_since, t)}
                  </span>
                  <button className="btn ghost" onClick={() => togglePending(task)}>
                    {t('task.clear')}
                  </button>
                </>
              ) : (
                <button className="btn ghost" onClick={() => togglePending(task)} title={t('pending.hint')}>
                  <Hourglass size={13} /> {t('pending.mark')}
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
                {task.body_md ? <Markdown remarkPlugins={MARKDOWN_PLUGINS} components={descriptionComponents}>{task.body_md}</Markdown> : t('task.addDescription')}
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

interface StartBranchProps {
  task: Task
  kind: string | null
  onKind: (k: string) => void
  /** What the user typed over the suggested name; null keeps the suggestion. */
  name: string | null
  onName: (name: string | null) => void
  onStart: (kind: string, name: string | null) => void
}

/** Start a conventional branch: pick the type, adjust the suggested name if you like, go. */
function StartBranch({ task, kind, onKind, name, onName, onStart }: StartBranchProps) {
  const { t } = useApp()
  const [suggestion] = useLive(() => api.suggestBranch(task.id, kind), [task.id, task.title, kind])
  const current = kind ?? suggestion?.[0] ?? 'feat'
  const value = name ?? suggestion?.[1] ?? ''
  const start = () => onStart(current, name)
  return (
    <div className="start-branch">
      <select
        className="input mono"
        value={current}
        onChange={(e) => {
          onKind(e.target.value)
          onName(null) // a new type means a new suggestion
        }}
        aria-label={t('git.branchType')}
        title={t('git.branchType')}
      >
        {BRANCH_TYPES.map((k) => (
          <option key={k} value={k}>
            {k}
          </option>
        ))}
      </select>
      <input
        className="input mono branch-name"
        value={value}
        onChange={(e) => onName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && value.trim()) start()
          else if (e.key === 'Escape' && name !== null) {
            e.stopPropagation()
            onName(null)
          }
        }}
        aria-label={t('git.branchName')}
        title={t('git.branchNameHint')}
        spellCheck={false}
      />
      <button className="btn" disabled={!value.trim()} onClick={start}>
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
  const [branchName, setBranchName] = useState<string | null>(null)
  const startBranch = (k: string | null, name: string | null) =>
    run(api.startBranch(task.id, k, name)).then((r) => r && toast(t('git.branchStarted', { branch: r.branch }), 'success'))
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
          'panel.branch': () => void startBranch(kind, branchName),
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
              <StartBranch task={task} kind={kind} onKind={setKind} name={branchName} onName={setBranchName} onStart={startBranch} />
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
