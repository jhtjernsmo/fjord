// Right-click menus plus confirm/prompt dialogs, shared through context.
import { createContext, useCallback, useContext, useEffect, useLayoutEffect, useRef, useState } from 'react'
import type { ReactNode } from 'react'
import { useApp } from '../data'
import { isTypingTarget } from '../keymap'

export interface MenuItem {
  label: string
  icon?: ReactNode
  danger?: boolean
  disabled?: boolean
  /** Draws a divider above this item. */
  separator?: boolean
  onSelect: () => void
}

interface ConfirmOptions {
  title: string
  message?: string
  confirmLabel: string
  danger?: boolean
}

interface PromptOptions {
  title: string
  label: string
  initial: string
  confirmLabel: string
}

interface MenusValue {
  openMenu: (e: { clientX: number; clientY: number; preventDefault: () => void }, items: MenuItem[]) => void
  confirm: (options: ConfirmOptions) => Promise<boolean>
  prompt: (options: PromptOptions) => Promise<string | null>
}

const MenusContext = createContext<MenusValue | null>(null)

export function useMenus(): MenusValue {
  const ctx = useContext(MenusContext)
  if (!ctx) throw new Error('useMenus must be used inside <MenuProvider>')
  return ctx
}

type Dialog =
  | { kind: 'confirm'; options: ConfirmOptions; resolve: (ok: boolean) => void }
  | { kind: 'prompt'; options: PromptOptions; resolve: (value: string | null) => void }

const EDGE_PX = 8

export function MenuProvider({ children }: { children: ReactNode }) {
  const [menu, setMenu] = useState<{ x: number; y: number; items: MenuItem[] } | null>(null)
  const [dialog, setDialog] = useState<Dialog | null>(null)

  // Replace the browser's own menu (Back, Reload, Inspect…) everywhere except text fields.
  useEffect(() => {
    const block = (e: MouseEvent) => {
      if (!isTypingTarget(e.target)) e.preventDefault()
    }
    window.addEventListener('contextmenu', block)
    return () => window.removeEventListener('contextmenu', block)
  }, [])

  const openMenu = useCallback<MenusValue['openMenu']>((e, items) => {
    e.preventDefault()
    setMenu({ x: e.clientX, y: e.clientY, items })
  }, [])

  const confirm = useCallback(
    (options: ConfirmOptions) => new Promise<boolean>((resolve) => setDialog({ kind: 'confirm', options, resolve })),
    [],
  )
  const prompt = useCallback(
    (options: PromptOptions) => new Promise<string | null>((resolve) => setDialog({ kind: 'prompt', options, resolve })),
    [],
  )

  return (
    <MenusContext.Provider value={{ openMenu, confirm, prompt }}>
      {children}
      {menu && <ContextMenu {...menu} onClose={() => setMenu(null)} />}
      {dialog?.kind === 'confirm' && (
        <ConfirmDialog
          options={dialog.options}
          onDone={(ok) => {
            dialog.resolve(ok)
            setDialog(null)
          }}
        />
      )}
      {dialog?.kind === 'prompt' && (
        <PromptDialog
          options={dialog.options}
          onDone={(value) => {
            dialog.resolve(value)
            setDialog(null)
          }}
        />
      )}
    </MenusContext.Provider>
  )
}

function ContextMenu({ x, y, items, onClose }: { x: number; y: number; items: MenuItem[]; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null)
  const [pos, setPos] = useState({ x, y })
  const [index, setIndex] = useState(-1)

  // Keep the menu on screen.
  useLayoutEffect(() => {
    const el = ref.current
    if (!el) return
    const { width, height } = el.getBoundingClientRect()
    setPos({
      x: Math.min(x, window.innerWidth - width - EDGE_PX),
      y: Math.min(y, window.innerHeight - height - EDGE_PX),
    })
  }, [x, y])

  useEffect(() => {
    const enabled = items.map((it, i) => (it.disabled ? -1 : i)).filter((i) => i >= 0)
    const onKey = (e: KeyboardEvent) => {
      e.stopPropagation()
      if (e.key === 'Escape') onClose()
      else if (e.key === 'ArrowDown' || e.key === 'j') setIndex((i) => enabled.find((n) => n > i) ?? enabled[0])
      else if (e.key === 'ArrowUp' || e.key === 'k') setIndex((i) => [...enabled].reverse().find((n) => n < i) ?? enabled[enabled.length - 1])
      else if (e.key === 'Enter' && index >= 0) {
        onClose()
        items[index].onSelect()
      } else return
      e.preventDefault()
    }
    const onDown = (e: MouseEvent) => !ref.current?.contains(e.target as Node) && onClose()
    window.addEventListener('keydown', onKey, true)
    window.addEventListener('mousedown', onDown)
    window.addEventListener('blur', onClose)
    window.addEventListener('resize', onClose)
    return () => {
      window.removeEventListener('keydown', onKey, true)
      window.removeEventListener('mousedown', onDown)
      window.removeEventListener('blur', onClose)
      window.removeEventListener('resize', onClose)
    }
  }, [items, index, onClose])

  return (
    <div ref={ref} className="menu context-menu" role="menu" style={{ left: pos.x, top: pos.y }}>
      {items.map((item, i) => (
        <div key={item.label} style={{ display: 'contents' }}>
          {item.separator && <div className="menu-sep" />}
          <button
            role="menuitem"
            className={`${item.danger ? 'danger' : ''} ${i === index ? 'on' : ''}`}
            disabled={item.disabled}
            onMouseEnter={() => setIndex(i)}
            onClick={() => {
              onClose()
              item.onSelect()
            }}
          >
            {item.icon} {item.label}
          </button>
        </div>
      ))}
    </div>
  )
}

function ConfirmDialog({ options, onDone }: { options: ConfirmOptions; onDone: (ok: boolean) => void }) {
  const { t } = useApp()
  return (
    <div className="overlay" onMouseDown={() => onDone(false)}>
      <div
        className="dialog"
        role="alertdialog"
        aria-label={options.title}
        onMouseDown={(e) => e.stopPropagation()}
        onKeyDown={(e) => e.key === 'Escape' && onDone(false)}
      >
        <h2>{options.title}</h2>
        {options.message && <p className="hint">{options.message}</p>}
        <div className="actions">
          <button className="btn ghost" onClick={() => onDone(false)}>
            {t('dialog.cancel')}
          </button>
          <button className={`btn ${options.danger ? 'danger-solid' : 'primary'}`} autoFocus onClick={() => onDone(true)}>
            {options.confirmLabel}
          </button>
        </div>
      </div>
    </div>
  )
}

function PromptDialog({ options, onDone }: { options: PromptOptions; onDone: (value: string | null) => void }) {
  const { t } = useApp()
  const [value, setValue] = useState(options.initial)
  return (
    <div className="overlay" onMouseDown={() => onDone(null)}>
      <form
        className="dialog"
        role="dialog"
        aria-label={options.title}
        onMouseDown={(e) => e.stopPropagation()}
        onSubmit={(e) => {
          e.preventDefault()
          if (value.trim()) onDone(value.trim())
        }}
        onKeyDown={(e) => e.key === 'Escape' && onDone(null)}
      >
        <h2>{options.title}</h2>
        <label className="row">
          {options.label}
          <input className="input" autoFocus value={value} onChange={(e) => setValue(e.target.value)} onFocus={(e) => e.currentTarget.select()} />
        </label>
        <div className="actions">
          <button type="button" className="btn ghost" onClick={() => onDone(null)}>
            {t('dialog.cancel')}
          </button>
          <button type="submit" className="btn primary" disabled={!value.trim()}>
            {options.confirmLabel}
          </button>
        </div>
      </form>
    </div>
  )
}
