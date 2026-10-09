// Azure Boards mentions: the last fetched list (shared by the runner and the
// Overview) and which ones the user has seen or dismissed.
import { useSyncExternalStore } from 'react'
import type { Mention } from './api'

const DISMISSED_KEY = 'fjord.dismissedMentions'
const SEEN_KEY = 'fjord.seenMentions'
/** Mentions older than Azure's 30-day window drop out, so this stays small. */
const MAX_REMEMBERED = 200

/** A new comment mentioning you on the same item counts as a new mention. */
export function mentionKey(m: Mention): string {
  return `${m.org.toLowerCase()}/${m.id}@${m.at}`
}

function readSet(key: string): Set<string> {
  try {
    const list: unknown = JSON.parse(localStorage.getItem(key) ?? '[]')
    return new Set(Array.isArray(list) ? list.filter((k): k is string => typeof k === 'string') : [])
  } catch {
    return new Set()
  }
}

function writeSet(key: string, set: Set<string>): void {
  try {
    localStorage.setItem(key, JSON.stringify([...set].slice(-MAX_REMEMBERED)))
  } catch {
    /* storage unavailable; the mention just shows again */
  }
}

let current: Mention[] = []
let dismissed = readSet(DISMISSED_KEY)
const listeners = new Set<() => void>()
const emit = () => listeners.forEach((l) => l())

function visible(): Mention[] {
  return current.filter((m) => !dismissed.has(mentionKey(m)))
}
let snapshot: Mention[] = []

/** Stores a fresh list and returns the mentions not seen before (to notify about). */
export function setMentions(list: Mention[]): Mention[] {
  current = list
  snapshot = visible()
  const seen = readSet(SEEN_KEY)
  const fresh = snapshot.filter((m) => !seen.has(mentionKey(m)))
  for (const m of list) seen.add(mentionKey(m))
  writeSet(SEEN_KEY, seen)
  emit()
  return fresh
}

export function dismissMention(m: Mention): void {
  dismissed = new Set(dismissed).add(mentionKey(m))
  writeSet(DISMISSED_KEY, dismissed)
  snapshot = visible()
  emit()
}

export function useMentions(): Mention[] {
  return useSyncExternalStore(
    (l) => {
      listeners.add(l)
      return () => listeners.delete(l)
    },
    () => snapshot,
  )
}
