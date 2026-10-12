import { describe, expect, test, vi } from 'vitest'
import { memo } from '../src/memo'

describe('memo', () => {
  test('reuses a result within the time limit and loads again after', async () => {
    const load = vi.fn(async () => 'x')
    await memo('a', load, 1000, 0)
    await memo('a', load, 1000, 500)
    expect(load).toHaveBeenCalledTimes(1)
    await memo('a', load, 1000, 1500)
    expect(load).toHaveBeenCalledTimes(2)
  })

  test('does not remember failures', async () => {
    const fail = vi.fn(async () => {
      throw new Error('offline')
    })
    await expect(memo('b', fail, 1000, 0)).rejects.toThrow('offline')
    await expect(memo('b', fail, 1000, 1)).rejects.toThrow('offline')
    expect(fail).toHaveBeenCalledTimes(2)
  })
})
