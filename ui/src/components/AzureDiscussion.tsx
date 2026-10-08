// Task panel: the Azure DevOps Discussion of an imported work item, collapsed
// until opened, then fetched (read-only).
import { useEffect, useState } from 'react'
import { ChevronRight, ExternalLink, MessagesSquare } from 'lucide-react'
import { api, errorMessage, relativeTime } from '../api'
import type { WorkItemComment } from '../api'
import { useApp, useLive } from '../data'
import { NoteMarkdown } from './NoteMarkdown'

export function AzureDiscussion({ taskId }: { taskId: number }) {
  const { t, locale } = useApp()
  const [link] = useLive(() => api.taskExternalLink(taskId), [taskId])
  const [open, setOpen] = useState(false)
  const [comments, setComments] = useState<WorkItemComment[] | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    setComments(null)
    setError(null)
  }, [taskId])

  useEffect(() => {
    if (!open || comments !== null) return
    api
      .azureDiscussion(taskId)
      .then((c) => setComments(c ?? []))
      .catch((e) => setError(errorMessage(e)))
  }, [open, comments, taskId])

  if (!link || link.source !== 'azure') return null

  return (
    <div className={`discussion ${open ? 'open' : ''}`}>
      <div className="section-title discussion-head">
        <button className="discussion-toggle" onClick={() => setOpen(!open)} aria-expanded={open}>
          <ChevronRight size={13} className="chevron" />
          <MessagesSquare size={13} /> {t('discussion.title')}
          {comments && <span className="dim"> ({comments.length})</span>}
        </button>
        <button className="icon-btn" onClick={() => api.openUrl(link.url)} title={t('discussion.openInAzure')} aria-label={t('discussion.openInAzure')}>
          <ExternalLink size={13} />
        </button>
      </div>
      {open && (
        <div className="discussion-body">
          {error && <div className="hint warn">{error}</div>}
          {!error && comments === null && <div className="hint">{t('github.checking')}</div>}
          {comments?.length === 0 && <div className="hint">{t('discussion.empty')}</div>}
          {comments?.map((c, i) => (
            <div key={i} className="comment">
              <div className="comment-head">
                <b>{c.author}</b> <span className="dim">{relativeTime(c.created, t, locale)}</span>
              </div>
              <div className="markdown">
                <NoteMarkdown markdown={c.text_md} />
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
