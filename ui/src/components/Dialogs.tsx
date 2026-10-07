import { useState } from 'react'
import type { NewProject } from '../api'

const COLORS = ['#7c9cff', '#00d4b0', '#3fb950', '#f5a524', '#ff7a1a', '#ff6b6b', '#e86bff', '#a78bfa']
const ICONS = ['📁', '🚀', '📱', '💸', '✈️', '🎨', '🛠️', '📚', '🧪', '🏔️', '🎮', '💡']

interface Props {
  onCancel: () => void
  onCreate: (input: NewProject) => void
}

export function NewProjectDialog({ onCancel, onCreate }: Props) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [color, setColor] = useState(COLORS[0])
  const [icon, setIcon] = useState(ICONS[1])

  const submit = () => name.trim() && onCreate({ name: name.trim(), description, color, icon })

  return (
    <div className="overlay" onMouseDown={onCancel}>
      <form
        className="dialog"
        onMouseDown={(e) => e.stopPropagation()}
        onSubmit={(e) => {
          e.preventDefault()
          submit()
        }}
        onKeyDown={(e) => e.key === 'Escape' && onCancel()}
        role="dialog"
        aria-label="Nytt prosjekt"
      >
        <h2>
          {icon} Nytt prosjekt
        </h2>
        <label className="row">
          Navn
          <input className="input" autoFocus value={name} onChange={(e) => setName(e.target.value)} placeholder="F.eks. Bokost" />
        </label>
        <label className="row">
          Beskrivelse
          <input className="input" value={description} onChange={(e) => setDescription(e.target.value)} placeholder="Valgfritt" />
        </label>
        <div className="row">
          Ikon
          <div className="emojis">
            {ICONS.map((i) => (
              <button type="button" key={i} className={`emoji ${i === icon ? 'on' : ''}`} onClick={() => setIcon(i)} aria-label={`Ikon ${i}`}>
                {i}
              </button>
            ))}
          </div>
        </div>
        <div className="row">
          Farge
          <div className="swatches">
            {COLORS.map((c) => (
              <button
                type="button"
                key={c}
                className={`swatch ${c === color ? 'on' : ''}`}
                style={{ background: c }}
                onClick={() => setColor(c)}
                aria-label={`Farge ${c}`}
              />
            ))}
          </div>
        </div>
        <div className="actions">
          <button type="button" className="btn ghost" onClick={onCancel}>
            Avbryt
          </button>
          <button type="submit" className="btn primary" disabled={!name.trim()}>
            Opprett <kbd>⏎</kbd>
          </button>
        </div>
      </form>
    </div>
  )
}
