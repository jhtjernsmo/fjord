// Small touches.
import { useEffect, useRef, useState } from 'react'
import type { ThemeDef } from './theme'

const decode = (s: string) => new TextDecoder().decode(Uint8Array.from(atob(s), (c) => c.charCodeAt(0)))

async function digest(s: string): Promise<string> {
  const buf = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(s))
  return Array.from(new Uint8Array(buf).slice(0, 8), (b) => b.toString(16).padStart(2, '0')).join('')
}

const LINES: Record<string, string> = {
  '8fe85f31aa6fa5ff': 'SGUgd29uIGFuIGF3YXJkIGZvciB0aGUgZmpvcmRzLg==',
  '3db31dea83e45a63': 'VGhpcyBpc24ndCB2aW0uIFNhdmVkIGFueXdheS4=',
  d61d8fde520b10bf: 'VGhpcyBpc24ndCB2aW0uIFNhdmVkIGFueXdheS4=',
  d13a3eed203bde31: 'TmljZSB0cnkuIFlvdSBhbHJlYWR5IGhhdmUgcGVybWlzc2lvbi4=',
}
const SEQUENCE = 'dd5866d170f7a3cb'
const SEQUENCE_LENGTH = 10

/** The line for a palette query, if it has one. */
export async function paletteLine(query: string): Promise<string | null> {
  const q = query.trim().toLowerCase().replace(/\s+/g, ' ')
  if (!q) return null
  const words = q.split(' ')
  for (const candidate of [q, words.length > 1 ? `${words[0]} *` : null]) {
    if (!candidate) continue
    const line = LINES[await digest(candidate)]
    if (line) return decode(line)
  }
  return null
}

export function usePaletteLine(query: string): string | null {
  const [line, setLine] = useState<string | null>(null)
  useEffect(() => {
    let live = true
    paletteLine(query).then((l) => live && setLine(l))
    return () => {
      live = false
    }
  }, [query])
  return line
}

export function cardTitle(id: number): string | undefined {
  return id === 6 * 7 ? decode('VGhlIGFuc3dlci4=') : undefined
}

export function lateLine(now = new Date()): string | null {
  const h = now.getHours()
  return h >= 2 && h < 5 ? decode('RG9uZS4gTm93IGdvIHRvIHNsZWVwLg==') : null
}

/** What the overview shows for the open count, and an extra line, on some days. */
export function dayFlair(now = new Date()): { open?: string; line?: string } {
  const m = now.getMonth() + 1
  const d = now.getDate()
  if (m === 3 && d === 14) return { open: Math.PI.toFixed(5) }
  if (m === 5 && d === 4) return { line: decode('TWF5IHRoZSA0dGggYmUgd2l0aCB5b3Uu') }
  return {}
}

export const FLAIR_THEME_ID = 'flair-p'

export const FLAIR_THEME: ThemeDef = {
  id: FLAIR_THEME_ID,
  name: decode('UGhvc3Bob3I='),
  dark: true,
  fontScale: 1,
  radius: 0,
  density: 'compact',
  builtin: false,
  colors: { bg: '#040904', bg2: '#071007', panel: '#0a150a', panel2: '#0e1c0e', line: '#143014', line2: '#1d451d', text: '#8cff8c', muted: '#62d062', dim: '#3c8c3c', accent: '#39ff14', ok: '#8cff8c', warn: '#e8ff6a', danger: '#ff6a5a' },
}

export const FLAIR_UNLOCKED = decode('UGhvc3Bob3IgdW5sb2NrZWQu')

/** Calls `onMatch` when the last few keys pressed anywhere form the sequence. */
export function useKeySequence(onMatch: () => void): void {
  const keys = useRef<string[]>([])
  const callback = useRef(onMatch)
  callback.current = onMatch
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey || e.altKey) return
      keys.current = [...keys.current, e.key.length === 1 ? e.key.toLowerCase() : e.key].slice(-SEQUENCE_LENGTH)
      if (keys.current.length < SEQUENCE_LENGTH) return
      digest(keys.current.join(' ')).then((h) => h === SEQUENCE && callback.current())
    }
    window.addEventListener('keydown', onKey, true)
    return () => window.removeEventListener('keydown', onKey, true)
  }, [])
}
