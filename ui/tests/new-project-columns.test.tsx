// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => null) }))

import { AppProvider } from '../src/data'
import { NewProjectDialog } from '../src/components/Dialogs'
import type { NewProject } from '../src/api'

afterEach(() => {
  cleanup()
  localStorage.clear()
})

async function renderDialog(onCreate: (input: NewProject) => void) {
  render(
    <AppProvider>
      <NewProjectDialog onCancel={() => {}} onCreate={onCreate} />
    </AppProvider>,
  )
  await act(async () => {
    await new Promise((r) => setTimeout(r, 20))
  })
}

describe('new project columns', () => {
  it("starts with Fjord's columns", async () => {
    const created: NewProject[] = []
    await renderDialog((input) => void created.push(input))
    fireEvent.change(screen.getByPlaceholderText('e.g. Bokost'), { target: { value: 'Sprint' } })
    await act(async () => {
      fireEvent.submit(screen.getByRole('dialog'))
    })
    expect(created[0].columns).toBe('fjord')
  })

  it('sends the Azure choice and keeps it for the next project', async () => {
    const created: NewProject[] = []
    await renderDialog((input) => void created.push(input))
    fireEvent.change(screen.getByPlaceholderText('e.g. Bokost'), { target: { value: 'Sprint' } })
    fireEvent.change(screen.getByLabelText('Columns'), { target: { value: 'azure' } })
    await act(async () => {
      fireEvent.submit(screen.getByRole('dialog'))
    })
    expect(created[0].columns).toBe('azure')

    cleanup()
    await renderDialog(() => {})
    expect((screen.getByLabelText('Columns') as HTMLSelectElement).value).toBe('azure')
  })
})
