// Overview: pull requests waiting for your review, replies on your own pull requests, and failed builds.
import { api, relativeTime } from '../api'
import type { WaitingItem } from '../api'
import { useApp, useLive } from '../data'
import { memo } from '../memo'

function WaitingRow({ item }: { item: WaitingItem }) {
  const { t, locale, run } = useApp()
  return (
    <button className="waiting-row" data-home-item data-home-action="open" onClick={() => run(api.openUrl(item.url))} title={item.url}>
      <span className={`waiting-kind waiting-${item.kind}`}>{t(`waiting.${item.kind}`)}</span>
      <span className="waiting-title">
        {item.number !== null && <span className="mono dim">#{item.number} </span>}
        {item.title}
      </span>
      <span className="waiting-repo">{item.repo}</span>
      <span className="waiting-by">{item.by ?? ''}</span>
      <span className="waiting-when">{relativeTime(item.at, t, locale)}</span>
    </button>
  )
}

export function WaitingOnYou() {
  const { t } = useApp()
  // Without a linked GitHub or Azure DevOps account the call fails; the section just stays hidden.
  const [items] = useLive(() => memo('waiting', () => api.waitingOnYou()).catch(() => [] as WaitingItem[]), [])
  if (!items || items.length === 0) return null
  return (
    <div>
      <div className="section-title" style={{ marginBottom: 8 }}>
        <span className="mentions-heading">
          {t('waiting.title')} <span className="count">{items.length}</span>
        </span>
      </div>
      <div className="due-list">
        {items.map((item) => (
          <WaitingRow key={item.url} item={item} />
        ))}
      </div>
    </div>
  )
}
