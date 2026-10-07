import { describe, expect, it, vi } from 'vitest'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => null) }))

import { linkTargets } from '../src/components/NoteMarkdown'

describe('linkTargets', () => {
  it('matches the core parser: targets in order, labels dropped, no duplicates', () => {
    const md = 'See [[Bokost]] and [[#12|the push bug]].\n[[ Launch plan ]] again [[Bokost]] [[]] [[broken\n]]'
    expect(linkTargets(md)).toEqual(['Bokost', '#12', 'Launch plan'])
    expect(linkTargets('no links here [x](y)')).toEqual([])
  })
})
