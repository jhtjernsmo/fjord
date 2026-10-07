// which-key popup (shown while a key sequence is pending) and the full help sheet.
import { useApp } from '../data'
import type { MessageKey } from '../i18n'
import { DEFAULT_ACTIONS, displayKeys } from '../keymap'

const PRETTY: Record<string, string> = { space: '␣', esc: 'Esc', enter: '⏎' }

export function WhichKey() {
  const { whichKey, pendingKeys, t } = useApp()
  if (!whichKey || pendingKeys.length === 0) return null
  // Collapse group continuations (e.g. "P" for "<leader>P a") into one entry.
  const seen = new Set<string>()
  const items = whichKey.filter((c) => !seen.has(c.key) && seen.add(c.key))
  const groupOf = (actionId: string) => DEFAULT_ACTIONS.find((a) => a.id === actionId)?.group ?? 'general'
  return (
    <div className="which-key" role="status">
      <div className="pending">{pendingKeys.map((k) => PRETTY[k] ?? k).join(' ')} …</div>
      <div className="grid">
        {items.map((c) => (
          <div key={c.key} className={`item ${c.isGroup ? 'group' : ''}`}>
            <span className="k">{PRETTY[c.key] ?? c.key}</span>
            <span className="l">
              {c.isGroup ? `+${t(`group.${groupOf(c.actionId)}` as MessageKey).toLowerCase()}` : t(`action.${c.actionId}` as MessageKey)}
            </span>
          </div>
        ))}
      </div>
    </div>
  )
}

export function HelpSheet({ onClose }: { onClose: () => void }) {
  const { keymap, t } = useApp()
  const groups = [...new Set(DEFAULT_ACTIONS.map((a) => a.group))]
  const [before, after] = t('help.intro').split('{leader}')
  return (
    <div className="overlay" onMouseDown={onClose}>
      <div className="help" onMouseDown={(e) => e.stopPropagation()} role="dialog" aria-label={t('help.title')}>
        <h2>{t('help.title')}</h2>
        <p className="intro">
          {before}
          <kbd>{displayKeys('<leader>', keymap.leader)}</kbd>
          {after} <code>~/.config/fjord/keymap.json</code>.
        </p>
        <div className="groups">
          {groups.map((g) => (
            <div className="group" key={g}>
              <h3>{t(`group.${g}` as MessageKey)}</h3>
              {DEFAULT_ACTIONS.filter((a) => a.group === g).map((a) => (
                <div className="row" key={a.id}>
                  <span>{t(`action.${a.id}` as MessageKey)}</span>
                  <kbd>{displayKeys(keymap.bindings[a.id], keymap.leader)}</kbd>
                </div>
              ))}
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
