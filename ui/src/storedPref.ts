// A per-device preference in localStorage that every component using it sees change at once,
// so a toggle in Settings shows up on the Overview behind it.
import { useCallback, useEffect, useState } from 'react'

const EVENT = 'fjord-pref'

export function readPref<T>(key: string, parse: (raw: unknown) => T, fallback: T): T {
  try {
    const raw = localStorage.getItem(key)
    return raw === null ? fallback : parse(JSON.parse(raw))
  } catch {
    return fallback
  }
}

/** `parse` and `fallback` should be module-level constants, not values created on each render. */
export function useStoredPref<T>(key: string, parse: (raw: unknown) => T, fallback: T): [T, (update: (current: T) => T) => void] {
  const [value, setValue] = useState(() => readPref(key, parse, fallback))
  useEffect(() => {
    const reload = (e: Event) => {
      if ((e as CustomEvent<string>).detail === key) setValue(readPref(key, parse, fallback))
    }
    window.addEventListener(EVENT, reload)
    return () => window.removeEventListener(EVENT, reload)
  }, [key, parse, fallback])
  const update = useCallback(
    (fn: (current: T) => T) => {
      try {
        localStorage.setItem(key, JSON.stringify(fn(readPref(key, parse, fallback))))
      } catch {
        /* non-fatal */
      }
      window.dispatchEvent(new CustomEvent(EVENT, { detail: key }))
    },
    [key, parse, fallback],
  )
  return [value, update]
}
