// which-key popup (shown while a key sequence is pending) and the full help sheet.
import { useApp } from '../data'
import { actionLabel, DEFAULT_ACTIONS, displayKeys } from '../keymap'

const PRETTY: Record<string, string> = { space: '␣', esc: 'Esc', enter: '⏎' }

export function WhichKey() {
  const { whichKey, pendingKeys } = useApp()
  if (!whichKey || pendingKeys.length === 0) return null
  // Collapse group continuations (e.g. "P" for "<leader>P a") into one entry.
  const seen = new Set<string>()
  const items = whichKey.filter((c) => !seen.has(c.key) && seen.add(c.key))
  return (
    <div className="which-key" role="status">
      <div className="pending">{pendingKeys.map((k) => PRETTY[k] ?? k).join(' ')} …</div>
      <div className="grid">
        {items.map((c) => (
          <div key={c.key} className={`item ${c.isGroup ? 'group' : ''}`}>
            <span className="k">{PRETTY[c.key] ?? c.key}</span>
            <span className="l">{c.isGroup ? `+${actionLabel(c.actionId).split(' ')[0].toLowerCase()}…` : actionLabel(c.actionId)}</span>
          </div>
        ))}
      </div>
    </div>
  )
}

export function HelpSheet({ onClose }: { onClose: () => void }) {
  const { keymap } = useApp()
  const groups = [...new Set(DEFAULT_ACTIONS.map((a) => a.group))]
  return (
    <div className="overlay" onMouseDown={onClose}>
      <div className="help" onMouseDown={(e) => e.stopPropagation()} role="dialog" aria-label="Hurtigtaster">
        <h2>Hurtigtaster</h2>
        <p className="intro">
          Alt kan gjøres med mus <em>eller</em> tastatur. Trykk <kbd>{displayKeys('<leader>', keymap.leader)}</kbd> og vent litt for å se
          valgene. Endre taster i <code>~/.config/fjord/keymap.json</code>.
        </p>
        <div className="groups">
          {groups.map((g) => (
            <div className="group" key={g}>
              <h3>{g}</h3>
              {DEFAULT_ACTIONS.filter((a) => a.group === g).map((a) => (
                <div className="row" key={a.id}>
                  <span>{a.label}</span>
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
