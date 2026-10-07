// Shared app state: live-refresh version counter, toasts, and the keymap engine.
import { createContext, useCallback, useContext, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import type { ReactNode } from 'react'
import { api, errorMessage } from './api'
import { initialLocale, saveLocale, translator } from './i18n'
import type { Locale, Translate } from './i18n'
import { buildKeymap, eventToken, isTypingTarget, match } from './keymap'
import type { Continuation, KeymapConfig, Scope } from './keymap'

const POLL_MS = 1500
const SEQUENCE_TIMEOUT_MS = 1200
const TOAST_MS = 3500

interface Toast {
  id: number
  text: string
  kind: 'info' | 'error' | 'success'
}

type Handler = () => void

interface AppContextValue {
  /** Bumps whenever the database changes (from this app, the CLI or an agent). */
  version: number
  refresh: () => void
  toast: (text: string, kind?: Toast['kind']) => void
  run: <T>(promise: Promise<T>, success?: string) => Promise<T | undefined>
  keymap: KeymapConfig
  register: (id: string, scope: Scope, fn: Handler) => () => void
  pendingKeys: string[]
  whichKey: Continuation[] | null
  locale: Locale
  setLocale: (locale: Locale) => void
  t: Translate
}

const AppContext = createContext<AppContextValue | null>(null)

export function useApp(): AppContextValue {
  const ctx = useContext(AppContext)
  if (!ctx) throw new Error('useApp must be used inside <AppProvider>')
  return ctx
}

export function AppProvider({ children }: { children: ReactNode }) {
  const [version, setVersion] = useState(0)
  const [toasts, setToasts] = useState<Toast[]>([])
  const [keymap, setKeymap] = useState<KeymapConfig>(() => buildKeymap(null))
  const [pendingKeys, setPendingKeys] = useState<string[]>([])
  const [whichKey, setWhichKey] = useState<Continuation[] | null>(null)
  const [locale, setLocaleState] = useState<Locale>(initialLocale)
  const t = useMemo(() => translator(locale), [locale])
  const setLocale = useCallback((next: Locale) => {
    saveLocale(next)
    setLocaleState(next)
  }, [])
  const lastCounter = useRef<number | null>(null)
  const handlers = useRef(new Map<string, { scope: Scope; fn: Handler }[]>())
  const pendingRef = useRef<string[]>([])
  const timer = useRef<number | undefined>(undefined)

  const refresh = useCallback(() => setVersion((v) => v + 1), [])

  const toast = useCallback((text: string, kind: Toast['kind'] = 'info') => {
    const id = Date.now() + Math.random()
    setToasts((list) => [...list, { id, text, kind }])
    window.setTimeout(() => setToasts((list) => list.filter((t) => t.id !== id)), TOAST_MS)
  }, [])

  const run = useCallback(
    async <T,>(promise: Promise<T>, success?: string) => {
      try {
        const value = await promise
        if (success) toast(success, 'success')
        refresh()
        return value
      } catch (err) {
        toast(errorMessage(err), 'error')
        return undefined
      }
    },
    [refresh, toast],
  )

  // Live updates: poll the activity counter so CLI/agent changes show up.
  useEffect(() => {
    const tick = async () => {
      try {
        const counter = await api.changeCounter()
        if (lastCounter.current !== null && counter !== lastCounter.current) refresh()
        lastCounter.current = counter
      } catch {
        /* backend not ready yet; try again next tick */
      }
    }
    tick()
    const id = window.setInterval(tick, POLL_MS)
    return () => window.clearInterval(id)
  }, [refresh])

  useEffect(() => {
    document.documentElement.lang = locale
  }, [locale])

  useEffect(() => {
    api
      .loadKeymap()
      .then((user) => setKeymap(buildKeymap(user)))
      .catch((err) => toast(`keymap.json: ${errorMessage(err)}`, 'error'))
  }, [toast])

  const register = useCallback((id: string, scope: Scope, fn: Handler) => {
    const entry = { scope, fn }
    handlers.current.set(id, [...(handlers.current.get(id) ?? []), entry])
    return () => handlers.current.set(id, (handlers.current.get(id) ?? []).filter((e) => e !== entry))
  }, [])

  // The key engine.
  useEffect(() => {
    const reset = () => {
      pendingRef.current = []
      setPendingKeys([])
      setWhichKey(null)
      window.clearTimeout(timer.current)
    }
    const activeScopes = (): Scope[] => {
      const scopes: Scope[] = ['global']
      if ((handlers.current.get('board.down') ?? []).length > 0) scopes.push('board')
      return scopes
    }
    const fire = (id: string) => {
      const list = handlers.current.get(id) ?? []
      list[list.length - 1]?.fn()
    }
    const onKey = (e: KeyboardEvent) => {
      const token = eventToken(e)
      if (!token) return
      const typing = isTypingTarget(e.target)
      // While typing, only Esc and ctrl/alt chords reach the engine.
      if (typing && token !== 'esc' && !token.includes('+')) return
      const pending = [...pendingRef.current, token]
      const result = match(pending, keymap, activeScopes())
      if (result.kind === 'none') {
        reset()
        return
      }
      e.preventDefault()
      if (result.kind === 'exact' && result.actionId) {
        reset()
        fire(result.actionId)
        return
      }
      pendingRef.current = pending
      setPendingKeys(pending)
      setWhichKey(result.next ?? null)
      window.clearTimeout(timer.current)
      timer.current = window.setTimeout(() => {
        // A prefix that is also a full binding (rare) fires on timeout.
        if (result.actionId) fire(result.actionId)
        reset()
      }, SEQUENCE_TIMEOUT_MS * (pending[0] === keymap.leader ? 3 : 1))
    }
    window.addEventListener('keydown', onKey)
    return () => {
      window.removeEventListener('keydown', onKey)
      window.clearTimeout(timer.current)
    }
  }, [keymap])

  const value = useMemo(
    () => ({ version, refresh, toast, run, keymap, register, pendingKeys, whichKey, locale, setLocale, t }),
    [version, refresh, toast, run, keymap, register, pendingKeys, whichKey, locale, setLocale, t],
  )

  return (
    <AppContext.Provider value={value}>
      {children}
      <div className="toasts" role="status" aria-live="polite">
        {toasts.map((t) => (
          <div key={t.id} className={`toast toast-${t.kind}`}>
            {t.text}
          </div>
        ))}
      </div>
    </AppContext.Provider>
  )
}

/** Loads data with `load` and reloads it whenever the app version changes. */
export function useLive<T>(load: () => Promise<T>, deps: unknown[]): [T | undefined, () => void] {
  const { version, toast } = useApp()
  const [data, setData] = useState<T>()
  const [local, setLocal] = useState(0)
  useEffect(() => {
    let cancelled = false
    load()
      .then((value) => !cancelled && setData(value))
      .catch((err) => !cancelled && toast(errorMessage(err), 'error'))
    return () => {
      cancelled = true
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [version, local, ...deps])
  return [data, () => setLocal((n) => n + 1)]
}

/** Binds action ids (see keymap.ts) to handlers while the component is mounted. */
export function useActions(map: Record<string, Handler>, scope: Scope = 'global'): void {
  const { register } = useApp()
  const latest = useRef(map)
  useLayoutEffect(() => {
    latest.current = map
  })
  const ids = Object.keys(map).sort().join('|')
  useEffect(() => {
    const unregister = ids
      .split('|')
      .filter(Boolean)
      .map((id) => register(id, scope, () => latest.current[id]?.()))
    return () => unregister.forEach((fn) => fn())
  }, [ids, scope, register])
}
