// @vitest-environment jsdom
import { act, renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it } from 'vitest'
import { useSidebarCollapsed } from '../src/sidebarPref'

describe('useSidebarCollapsed', () => {
  beforeEach(() => localStorage.clear())

  it('starts expanded and remembers a toggle', () => {
    const { result, unmount } = renderHook(() => useSidebarCollapsed())
    expect(result.current[0]).toBe(false)

    act(() => result.current[1]())
    expect(result.current[0]).toBe(true)
    unmount()

    const again = renderHook(() => useSidebarCollapsed())
    expect(again.result.current[0]).toBe(true)
  })

  it('expands again on a second toggle', () => {
    localStorage.setItem('fjord.sidebarCollapsed', '1')
    const { result } = renderHook(() => useSidebarCollapsed())

    act(() => result.current[1]())

    expect(result.current[0]).toBe(false)
    expect(localStorage.getItem('fjord.sidebarCollapsed')).toBe('0')
  })
})
