import { describe, expect, it } from 'vitest'
import { buildKeymap, DEFAULT_ACTIONS, displayKeys, eventToken, match, parseSequence } from '../src/keymap'

const key = (k: string, mods: Partial<KeyboardEventInit> = {}) => ({ key: k, ctrlKey: false, altKey: false, metaKey: false, shiftKey: false, ...mods }) as KeyboardEvent

describe('parseSequence', () => {
  it('expands the leader and keeps single-char case', () => {
    expect(parseSequence('<leader>P a', 'space')).toEqual(['space', 'P', 'a'])
    expect(parseSequence('ctrl+K', 'space')).toEqual(['ctrl+k'])
    expect(parseSequence('g g', ',')).toEqual(['g', 'g'])
  })
})

describe('eventToken', () => {
  it('normalizes named keys, modifiers and shifted letters', () => {
    expect(eventToken(key(' '))).toBe('space')
    expect(eventToken(key('Escape'))).toBe('esc')
    expect(eventToken(key('J', { shiftKey: true }))).toBe('J')
    expect(eventToken(key('K', { ctrlKey: true, shiftKey: true }))).toBe('ctrl+k')
    expect(eventToken(key('Enter', { shiftKey: true }))).toBe('shift+enter')
    expect(eventToken(key('Shift', { shiftKey: true }))).toBeNull()
  })

  it('treats Cmd as Ctrl on macOS only', () => {
    expect(eventToken(key('k', { metaKey: true }), true)).toBe('ctrl+k')
    expect(eventToken(key('k', { metaKey: true }), false)).toBe('meta+k')
    expect(displayKeys('ctrl+k', 'space', true)).toBe('⌘k')
    expect(displayKeys('ctrl+k', 'space', false)).toBe('ctrl+k')
  })
})

describe('buildKeymap', () => {
  it('merges valid user overrides and ignores junk', () => {
    const km = buildKeymap({ leader: ',', bindings: { 'task.archive': 'D', 'not.an.action': 'x', 'board.down': 42 } })
    expect(km.leader).toBe(',')
    expect(km.bindings['task.archive']).toBe('D')
    expect(km.bindings['board.down']).toBe('j')
    expect(km.bindings).not.toHaveProperty('not.an.action')
    expect(buildKeymap(null).leader).toBe('space')
    expect(buildKeymap('garbage').bindings['go.home']).toBe('<leader>h')
  })
})

describe('match', () => {
  const km = buildKeymap(null)

  it('finds exact bindings, prefixes and groups by scope', () => {
    expect(match(['j'], km, ['global', 'board'])).toEqual({ kind: 'exact', actionId: 'board.down' })
    expect(match(['j'], km, ['global']).kind).toBe('none')
    const leader = match(['space'], km, ['global'])
    expect(leader.kind).toBe('prefix')
    expect(leader.next?.find((c) => c.key === 'P')?.isGroup).toBe(true)
    expect(match(['space', 'P', 'a'], km, ['global'])).toEqual({ kind: 'exact', actionId: 'project.archive' })
    expect(match(['g'], km, ['board']).kind).toBe('prefix')
    expect(match(['g', 'g'], km, ['board']).actionId).toBe('board.first')
  })

  it('has no conflicting default bindings within a scope set', () => {
    const scopes = [['global'], ['global', 'board']] as const
    for (const active of scopes) {
      const seen = new Map<string, string>()
      for (const a of DEFAULT_ACTIONS.filter((x) => (active as readonly string[]).includes(x.scope))) {
        const seq = parseSequence(km.bindings[a.id], km.leader).join(' ')
        expect(seen.get(seq), `${a.id} clashes with ${seen.get(seq)}`).toBeUndefined()
        seen.set(seq, a.id)
      }
    }
  })
})

describe('displayKeys', () => {
  it('prettifies special keys', () => {
    expect(displayKeys('<leader>f', 'space', false)).toBe('␣ f')
    expect(displayKeys('enter', 'space', false)).toBe('⏎')
  })
})
