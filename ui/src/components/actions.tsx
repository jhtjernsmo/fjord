// Shared entity actions (rename, delete, archive …) and the right-click menus built from them.
import { Archive, ArchiveRestore, ArrowRight, ExternalLink, Flag, FolderOpen, GitBranch, Pencil, Trash2, X } from 'lucide-react'
import { api } from '../api'
import type { Attachment, Note, Project, Status, Task } from '../api'
import { useApp } from '../data'
import type { MessageKey } from '../i18n'
import { useMenus } from './Menus'
import type { MenuItem } from './Menus'

const ICON = 14

export function useEntityActions() {
  const { run, t } = useApp()
  const { confirm, prompt } = useMenus()

  const renameTask = async (task: Task) => {
    const title = await prompt({ title: t('menu.renameTask'), label: t('task.title'), initial: task.title, confirmLabel: t('menu.rename') })
    if (title && title !== task.title) await run(api.updateTask(task.id, { title }))
  }

  const deleteTask = async (task: Task, onDeleted?: () => void) => {
    const ok = await confirm({
      title: t('confirm.deleteTask', { name: task.title }),
      message: t('confirm.permanent'),
      confirmLabel: t('menu.delete'),
      danger: true,
    })
    if (ok && (await run(api.deleteTask(task.id).then(() => true), t('toast.deleted', { name: task.title })))) onDeleted?.()
  }

  const renameProject = async (project: Project) => {
    const name = await prompt({ title: t('menu.renameProject'), label: t('dialog.name'), initial: project.name, confirmLabel: t('menu.rename') })
    if (name && name !== project.name) await run(api.updateProject(project.id, { name }))
  }

  const archiveProject = async (project: Project, archived: boolean, onDone?: () => void) => {
    const msg = archived ? t('toast.archivedProject', { name: project.name }) : t('archive.restored', { name: project.name })
    if (await run(api.archiveProject(project.id, archived).then(() => true), msg)) onDone?.()
  }

  const deleteProject = async (project: Project, onDeleted?: () => void) => {
    const ok = await confirm({
      title: t('confirm.deleteProject', { name: project.name }),
      message: t('confirm.deleteProjectHint'),
      confirmLabel: t('menu.delete'),
      danger: true,
    })
    if (ok && (await run(api.deleteProject(project.id).then(() => true), t('toast.deleted', { name: project.name })))) onDeleted?.()
  }

  const renameNote = async (note: Note) => {
    const title = await prompt({ title: t('menu.renameNote'), label: t('task.title'), initial: note.title, confirmLabel: t('menu.rename') })
    if (title && title !== note.title) await run(api.updateNote(note.id, title, note.body_md))
  }

  const deleteNote = async (note: Note) => {
    const ok = await confirm({ title: t('confirm.deleteNote', { name: note.title }), message: t('confirm.permanent'), confirmLabel: t('menu.delete'), danger: true })
    if (ok) await run(api.deleteNote(note.id), t('toast.deleted', { name: note.title }))
  }

  return { renameTask, deleteTask, renameProject, archiveProject, deleteProject, renameNote, deleteNote }
}

/** Menus for each kind of item. Each returns a function you can pass to onContextMenu. */
export function useContextMenus() {
  const { run, t, toast } = useApp()
  const { openMenu } = useMenus()
  const actions = useEntityActions()

  const taskMenu = (task: Task, statuses: Status[], opts: { onOpen: () => void; hasRepo?: boolean }) => (e: React.MouseEvent) => {
    const others = statuses.filter((s) => s.id !== task.status_id)
    const items: MenuItem[] = [
      { label: t('menu.open'), icon: <FolderOpen size={ICON} />, onSelect: opts.onOpen },
      { label: t('menu.rename'), icon: <Pencil size={ICON} />, onSelect: () => actions.renameTask(task) },
      ...others.map((s, i) => ({
        label: t('menu.moveTo', { column: s.name }),
        icon: <ArrowRight size={ICON} />,
        separator: i === 0,
        onSelect: () => run(api.moveTask(task.id, s.id)),
      })),
      ...[3, 2, 1, 0]
        .filter((p) => p !== task.priority)
        .map((p, i) => ({
          label: t('menu.priority', { p: t(`priority.${p}` as MessageKey) }),
          icon: <Flag size={ICON} />,
          separator: i === 0,
          onSelect: () => run(api.updateTask(task.id, { priority: p })),
        })),
    ]
    if (opts.hasRepo) {
      items.push({
        label: t('git.startBranch'),
        icon: <GitBranch size={ICON} />,
        separator: true,
        onSelect: () => run(api.startBranch(task.id)).then((r) => r && toast(t('git.branchStarted', { branch: r.branch }), 'success')),
      })
    }
    items.push(
      { label: t('task.archive'), icon: <Archive size={ICON} />, separator: !opts.hasRepo, onSelect: () => run(api.archiveTask(task.id, true), t('toast.archivedTask', { name: task.title })) },
      { label: t('menu.delete'), icon: <Trash2 size={ICON} />, danger: true, onSelect: () => actions.deleteTask(task) },
    )
    openMenu(e, items)
  }

  const projectMenu = (project: Project, opts: { onOpen: () => void; onGone?: () => void }) => (e: React.MouseEvent) => {
    const archived = !!project.archived_at
    openMenu(e, [
      { label: t('menu.open'), icon: <FolderOpen size={ICON} />, onSelect: opts.onOpen, disabled: archived },
      { label: t('menu.rename'), icon: <Pencil size={ICON} />, onSelect: () => actions.renameProject(project) },
      archived
        ? { label: t('archive.restore'), icon: <ArchiveRestore size={ICON} />, separator: true, onSelect: () => actions.archiveProject(project, false) }
        : { label: t('task.archive'), icon: <Archive size={ICON} />, separator: true, onSelect: () => actions.archiveProject(project, true, opts.onGone) },
      { label: t('menu.delete'), icon: <Trash2 size={ICON} />, danger: true, onSelect: () => actions.deleteProject(project, opts.onGone) },
    ])
  }

  const noteMenu = (note: Note, onOpen: () => void) => (e: React.MouseEvent) =>
    openMenu(e, [
      { label: t('menu.open'), icon: <FolderOpen size={ICON} />, onSelect: onOpen },
      { label: t('menu.rename'), icon: <Pencil size={ICON} />, onSelect: () => actions.renameNote(note) },
      { label: t('menu.delete'), icon: <Trash2 size={ICON} />, danger: true, separator: true, onSelect: () => actions.deleteNote(note) },
    ])

  const fileMenu = (file: Attachment) => (e: React.MouseEvent) =>
    openMenu(e, [
      { label: t('file.open'), icon: <ExternalLink size={ICON} />, onSelect: () => run(api.openAttachment(file.id)) },
      { label: t('file.remove'), icon: <X size={ICON} />, danger: true, separator: true, onSelect: () => run(api.detachFile(file.id)) },
    ])

  const archivedTaskMenu = (task: Task) => (e: React.MouseEvent) =>
    openMenu(e, [
      { label: t('archive.restore'), icon: <ArchiveRestore size={ICON} />, onSelect: () => run(api.archiveTask(task.id, false), t('archive.restored', { name: task.title })) },
      { label: t('menu.delete'), icon: <Trash2 size={ICON} />, danger: true, separator: true, onSelect: () => actions.deleteTask(task) },
    ])

  return { taskMenu, projectMenu, noteMenu, fileMenu, archivedTaskMenu }
}
