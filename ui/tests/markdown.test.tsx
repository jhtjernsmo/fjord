// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, render, screen, waitFor } from '@testing-library/react'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => null) }))
const mermaid = vi.hoisted(() => ({
  initialize: vi.fn(),
  render: vi.fn(async (_id: string, code: string) => {
    if (code.includes('oops')) throw new Error('Parse error on line 2')
    return { svg: `<svg data-test="diagram"><text>${code.length}</text></svg>` }
  }),
}))
vi.mock('mermaid', () => ({ default: mermaid }))

import { AppProvider } from '../src/data'
import { ThemeProvider } from '../src/themes'
import { NoteMarkdown } from '../src/components/NoteMarkdown'

afterEach(cleanup)

const show = (markdown: string) =>
  render(
    <ThemeProvider>
      <AppProvider>
        <NoteMarkdown markdown={markdown} />
      </AppProvider>
    </ThemeProvider>,
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

  it('draws ```mermaid blocks as diagrams and leaves other code alone', async () => {
    const { container } = show('```mermaid\ngraph TD\n  A-->B\n```\n\n```js\nconst x = 1\n```')
    await waitFor(() => expect(container.querySelector('.mermaid-diagram svg')).not.toBeNull(), { timeout: 2000 })
    expect(mermaid.render).toHaveBeenCalledWith(expect.any(String), 'graph TD\n  A-->B\n')
    expect(container.querySelector('pre code')?.textContent).toContain('const x = 1')
  })

  it('shows the error and the source when a diagram has a mistake', async () => {
    const { container } = show('```mermaid\ngraph TD\n  oops -->\n```')
    await waitFor(() => expect(container.querySelector('.mermaid-error')).not.toBeNull(), { timeout: 2000 })
    expect(container.textContent).toContain('Parse error on line 2')
    expect(container.querySelector('.mermaid-error pre')?.textContent).toContain('oops')
  })
})
