// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, render, screen } from '@testing-library/react'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => null) }))

import { AppProvider } from '../src/data'
import { NoteMarkdown } from '../src/components/NoteMarkdown'

afterEach(cleanup)

const show = (markdown: string) =>
  render(
    <AppProvider>
      <NoteMarkdown markdown={markdown} />
    </AppProvider>,
  )

describe('NoteMarkdown (GitHub-flavored)', () => {
  it('renders tables, keeping [[target|label]] links inside cells intact', () => {
    const { container } = show('| Item | Amount |\n|---|---:|\n| [[Rent|Housing]] | 1 000 |\n| **Sum** | **1 000** |')
    const table = container.querySelector('.md-table table')
    expect(table).not.toBeNull()
    expect(table?.querySelectorAll('tbody tr')).toHaveLength(2)
    expect(screen.getByRole('columnheader', { name: 'Amount' }).style.textAlign).toBe('right')
    expect(screen.getByText('Housing')).toBeTruthy()
    expect(container.textContent).not.toContain('|')
  })

  it('renders task lists and strikethrough', () => {
    const { container } = show('- [x] done\n- [ ] todo\n\n~~old~~')
    const boxes = container.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')
    expect([...boxes].map((b) => b.checked)).toEqual([true, false])
    expect(container.querySelector('del')?.textContent).toBe('old')
  })
})
