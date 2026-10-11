// Whether the sidebar is collapsed to icons. A per-device preference kept in localStorage, like the theme.
import { useCallback, useEffect, useState } from 'react'

const KEY = 'fjord.sidebarCollapsed'

function readCollapsed(): boolean {
  try {
    return localStorage.getItem(KEY) === '1'
  } catch {
    return false
  }
}

export function useSidebarCollapsed(): [boolean, () => void] {
  const [collapsed, setCollapsed] = useState(readCollapsed)
  useEffect(() => {
    try {
      localStorage.setItem(KEY, collapsed ? '1' : '0')
    } catch {
      /* non-fatal */
    }
  }, [collapsed])
  const toggle = useCallback(() => setCollapsed((c) => !c), [])
  return [collapsed, toggle]
}
