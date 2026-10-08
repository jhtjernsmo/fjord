import { useState } from 'react'
import { useApp } from '../data'
import { EDITOR_PRESETS, editorCommand, loadEditor, saveEditor, type EditorChoice } from '../editor'

/** Settings → Integrations: which editor "Open in editor" starts. */
export function EditorSettings() {
  const { t } = useApp()
  const [choice, setChoice] = useState<EditorChoice>(loadEditor)
  const update = (next: EditorChoice) => {
    setChoice(next)
    saveEditor(next)
  }
  return (
    <div className="row">
      {t('editor.title')}
      <select className="input" value={choice.id} onChange={(e) => update({ ...choice, id: e.target.value })}>
        {EDITOR_PRESETS.map((p) => (
          <option key={p.id} value={p.id}>
            {p.label}
          </option>
        ))}
        <option value="custom">{t('editor.custom')}</option>
      </select>
      {choice.id === 'custom' && (
        <input
          className="input mono"
          value={choice.custom}
          placeholder={'"C:\\Program Files\\My IDE\\ide.exe" {path}'}
          onChange={(e) => update({ ...choice, custom: e.target.value })}
          aria-label={t('editor.customCommand')}
        />
      )}
      <span className="hint">
        {t('editor.hint')} <code className="mono">{editorCommand(choice) || '…'}</code>
      </span>
    </div>
  )
}
