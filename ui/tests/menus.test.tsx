// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => null) }))

import { AppProvider } from '../src/data'
import { MenuProvider, useMenus } from '../src/components/Menus'

function Probe({ onRename, onConfirmed }: { onRename: () => void; onConfirmed: (ok: boolean) => void }) {
  const { openMenu, confirm } = useMenus()
  return (
    <>
      <div data-testid="card" onContextMenu={(e) => openMenu(e, [{ label: 'Rename', onSelect: onRename }])}>
        card
      </div>
      <button onClick={() => confirm({ title: 'Delete it?', confirmLabel: 'Delete', danger: true }).then(onConfirmed)}>ask</button>
      <input aria-label="field" />
    </>
  )
}

function setup() {
  const onRename = vi.fn()
  const onConfirmed = vi.fn()
  render(
    <AppProvider>
      <MenuProvider>
        <Probe onRename={onRename} onConfirmed={onConfirmed} />
      </MenuProvider>
    </AppProvider>,
  )
  return { onRename, onConfirmed }
}

afterEach(cleanup)

describe('menus', () => {
  it('replaces the browser menu with our own, but not in text fields', () => {
    const { onRename } = setup()
    const blank = new MouseEvent('contextmenu', { bubbles: true, cancelable: true })
    document.body.dispatchEvent(blank)
    expect(blank.defaultPrevented).toBe(true)

    const inField = new MouseEvent('contextmenu', { bubbles: true, cancelable: true })
    screen.getByLabelText('field').dispatchEvent(inField)
    expect(inField.defaultPrevented).toBe(false)

    fireEvent.contextMenu(screen.getByTestId('card'), { clientX: 10, clientY: 10 })
    fireEvent.click(screen.getByRole('menuitem', { name: /Rename/ }))
    expect(onRename).toHaveBeenCalledTimes(1)
    expect(screen.queryByRole('menu')).toBeNull()
  })

  it('confirm resolves true on confirm and false on cancel', async () => {
    const { onConfirmed } = setup()
    fireEvent.click(screen.getByText('ask'))
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Delete' }))
    })
    expect(onConfirmed).toHaveBeenLastCalledWith(true)

    fireEvent.click(screen.getByText('ask'))
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    })
    expect(onConfirmed).toHaveBeenLastCalledWith(false)
  })
})
