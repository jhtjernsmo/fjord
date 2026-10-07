// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { act, cleanup, render, screen } from '@testing-library/react'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string) => {
    if (cmd === 'change_counter') return 1
    if (cmd === 'load_keymap') return { bindings: { 'task.archive': 'D' } }
    return null
  }),
}))

import { AppProvider, useActions } from '../src/data'
import { WhichKey } from '../src/components/WhichKey'

function press(key: string, init: KeyboardEventInit = {}) {
  act(() => {
    window.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, ...init }))
  })
}

function Probe({ onFirst, onArchive, onHome }: { onFirst: () => void; onArchive: () => void; onHome: () => void }) {
  useActions({ 'board.first': onFirst, 'board.down': () => undefined, 'task.archive': onArchive }, 'board')
  useActions({ 'go.home': onHome })
  return (
    <>
      <input aria-label="typing" />
      <WhichKey />
    </>
  )
}

afterEach(cleanup)

describe('key engine', () => {
  it('runs multi-key sequences, user overrides and leader bindings', async () => {
    const onFirst = vi.fn()
    const onArchive = vi.fn()
    const onHome = vi.fn()
    render(
      <AppProvider>
        <Probe onFirst={onFirst} onArchive={onArchive} onHome={onHome} />
      </AppProvider>,
    )
    await act(async () => {
      await Promise.resolve()
    })

    press('g')
    press('g')
    expect(onFirst).toHaveBeenCalledTimes(1)

    // keymap.json override replaced "d d" with "D"
    press('D', { shiftKey: true })
    expect(onArchive).toHaveBeenCalledTimes(1)

    press(' ')
    expect(document.querySelector('.which-key')?.textContent).toContain('Home / overview')
    press('h')
    expect(onHome).toHaveBeenCalledTimes(1)
  })

  it('ignores plain keys while typing in an input', async () => {
    const onFirst = vi.fn()
    render(
      <AppProvider>
        <Probe onFirst={onFirst} onArchive={vi.fn()} onHome={vi.fn()} />
      </AppProvider>,
    )
    const input = screen.getByLabelText('typing')
    input.focus()
    act(() => {
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'g', bubbles: true }))
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'g', bubbles: true }))
    })
    expect(onFirst).not.toHaveBeenCalled()
  })
})
