// Fjord's own title bar: continues the sidebar and main area up to the top edge,
// drags the window, and draws minimize / maximize / close on Windows and Linux.
// macOS keeps its native traffic lights (overlaid, see tauri.macos.conf.json).
import { useEffect, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Copy, Minus, Square, X } from 'lucide-react'
import { useApp } from '../data'

export const IS_MAC = typeof navigator !== 'undefined' && /Mac/i.test(navigator.userAgent)

function useMaximized(): boolean {
  const [maximized, setMaximized] = useState(false)
  useEffect(() => {
    const win = getCurrentWindow()
    const sync = () => win.isMaximized().then(setMaximized).catch(() => undefined)
    sync()
    let unlisten: (() => void) | undefined
    win
      .onResized(sync)
      .then((fn) => (unlisten = fn))
      .catch(() => undefined)
    return () => unlisten?.()
  }, [])
  return maximized
}

function WindowControls() {
  const { t } = useApp()
  const maximized = useMaximized()
  const win = getCurrentWindow()
  const run = (action: () => Promise<void>) => () => void action().catch(() => undefined)
  return (
    <div className="window-controls">
      <button onClick={run(() => win.minimize())} aria-label={t('window.minimize')} title={t('window.minimize')}>
        <Minus size={15} strokeWidth={1.4} />
      </button>
      <button onClick={run(() => win.toggleMaximize())} aria-label={t('window.maximize')} title={maximized ? t('window.restore') : t('window.maximize')}>
        {maximized ? <Copy size={12} strokeWidth={1.4} style={{ transform: 'scaleX(-1)' }} /> : <Square size={12} strokeWidth={1.4} />}
      </button>
      <button className="close" onClick={run(() => win.close())} aria-label={t('window.close')} title={t('window.close')}>
        <X size={16} strokeWidth={1.4} />
      </button>
    </div>
  )
}

/** `title` is shown faintly in the middle, like a document name. */
export function TitleBar({ title }: { title?: string }) {
  return (
    <div className={`titlebar ${IS_MAC ? 'mac' : ''}`} data-tauri-drag-region>
      <div className="titlebar-side" data-tauri-drag-region />
      <div className="titlebar-main" data-tauri-drag-region>
        <span className="titlebar-title" data-tauri-drag-region>
          {title}
        </span>
        {!IS_MAC && <WindowControls />}
      </div>
    </div>
  )
}
