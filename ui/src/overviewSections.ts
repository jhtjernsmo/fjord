// Which sections the Overview shows. Each can be turned off in Settings; the work-specific ones start off.
import { useCallback } from 'react'
import { useStoredPref } from './storedPref'

export const OVERVIEW_SECTIONS = ['standup', 'due', 'pending', 'waiting', 'mentions', 'projects', 'activity'] as const
export type OverviewSection = (typeof OVERVIEW_SECTIONS)[number]
export type OverviewSections = Record<OverviewSection, boolean>

export const DEFAULT_SECTIONS: OverviewSections = { standup: false, due: true, pending: true, waiting: false, mentions: true, projects: true, activity: true }

function parseSections(raw: unknown): OverviewSections {
  const saved = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : {}
  return Object.fromEntries(OVERVIEW_SECTIONS.map((id) => [id, typeof saved[id] === 'boolean' ? saved[id] : DEFAULT_SECTIONS[id]])) as OverviewSections
}

export function useOverviewSections(): [OverviewSections, (id: OverviewSection, on: boolean) => void] {
  const [sections, update] = useStoredPref('fjord.overviewSections', parseSections, DEFAULT_SECTIONS)
  const set = useCallback((id: OverviewSection, on: boolean) => update((s) => ({ ...s, [id]: on })), [update])
  return [sections, set]
}
