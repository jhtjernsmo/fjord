// @vitest-environment jsdom
import { beforeEach, describe, expect, test, vi } from 'vitest'
import type { Mention } from '../src/api'

const mention = (id: number, at = '2026-10-09T08:00:00Z'): Mention => ({
  org: 'Contoso',
  id,
  project: 'App',
  kind: 'Bug',
  state: 'Active',
  title: `Item ${id}`,
  url: `https://dev.azure.com/contoso/App/_workitems/edit/${id}`,
  by: 'Kari',
  at,
  snippet: '@Jonas can you look?',
})

// The store keeps module state, so each test gets a fresh copy.
async function freshStore() {
  vi.resetModules()
  return import('../src/mentions')
}

describe('mentions store', () => {
  beforeEach(() => localStorage.clear())

  test('reports a mention as new only the first time it is seen', async () => {
    const { setMentions } = await freshStore()
    expect(setMentions([mention(1), mention(2)]).map((m) => m.id)).toEqual([1, 2])
    expect(setMentions([mention(1), mention(2)])).toEqual([])
    expect(setMentions([mention(1), mention(3)]).map((m) => m.id)).toEqual([3])
  })

  test('remembers what was seen across restarts', async () => {
    ;(await freshStore()).setMentions([mention(1)])
    expect((await freshStore()).setMentions([mention(1)])).toEqual([])
  })

  test('a dismissed mention stays hidden until someone mentions you again', async () => {
    const { dismissMention, mentionKey, setMentions } = await freshStore()
    setMentions([mention(1)])
    dismissMention(mention(1))
    expect(setMentions([mention(1)])).toEqual([])
    const again = mention(1, '2026-10-10T09:00:00Z')
    expect(mentionKey(again)).not.toBe(mentionKey(mention(1)))
    expect(setMentions([again]).map((m) => m.at)).toEqual([again.at])
  })
})
