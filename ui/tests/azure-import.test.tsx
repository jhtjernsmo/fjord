// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'

// A tiny fake backend: saving bumps the change counter, like a real write does.
const backend = vi.hoisted(() => ({ saved: { enabled: false, mappings: [] as unknown[] }, counter: 0 }))
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string, args?: { settings?: typeof backend.saved }) => {
    if (cmd === 'get_import_settings') return structuredClone(backend.saved)
    if (cmd === 'set_import_settings' && args?.settings) {
      backend.saved = args.settings
      backend.counter += 1
      return null
    }
    if (cmd === 'list_projects') return [{ id: 1, name: 'Bokost' }]
    if (cmd === 'change_counter') return backend.counter
    return null
  }),
}))

import { AppProvider } from '../src/data'
import { AzureImportSettings } from '../src/components/AzureImport'

afterEach(cleanup)

describe('Azure Boards import settings', () => {
  it('keeps a half-filled mapping row while you move between its fields', async () => {
    render(
      <AppProvider>
        <AzureImportSettings />
      </AppProvider>,
    )
    await act(async () => {
      await new Promise((r) => setTimeout(r, 50))
    })
    fireEvent.click(screen.getByText('Add project'))
    const org = screen.getByLabelText('organization')
    fireEvent.change(org, { target: { value: 'contoso' } })
    // Leaving the org field saves; the incomplete row isn't persisted yet…
    await act(async () => {
      fireEvent.blur(org)
      await new Promise((r) => setTimeout(r, 1200))
    })
    // …but it must still be on screen so the Azure project can be filled in.
    const project = screen.getByLabelText('Azure project') as HTMLInputElement
    expect((screen.getByLabelText('organization') as HTMLInputElement).value).toBe('contoso')
    fireEvent.change(project, { target: { value: 'Mobile App' } })
    await act(async () => {
      fireEvent.blur(project)
      await new Promise((r) => setTimeout(r, 50))
    })
    expect(backend.saved.mappings).toEqual([{ org: 'contoso', project: 'Mobile App', fjord_project_id: 1 }])
  })
})
