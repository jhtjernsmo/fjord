// `y` on a task: copy a [[#12]] reference to paste into a note.
import type { Task } from '../api'
import type { MessageKey } from '../i18n'

type Translate = (key: MessageKey, vars?: Record<string, string | number>) => string

export function copyTaskRef(task: Task, toast: (text: string, kind?: 'success' | 'error') => void, t: Translate): void {
  const ref = `[[#${task.id}]]`
  navigator.clipboard
    ?.writeText(ref)
    .then(() => toast(t('task.refCopied', { ref }), 'success'))
    .catch(() => toast(ref, 'success'))
}
