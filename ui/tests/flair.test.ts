import { describe, expect, test } from 'vitest'
import { cardTitle, dayFlair, lateLine, paletteLine } from '../src/flair'

const q = (b64: string) => atob(b64)

describe('palette lines', () => {
  test('ordinary queries have none', async () => {
    expect(await paletteLine('')).toBeNull()
    expect(await paletteLine('new task')).toBeNull()
  })

  test('known queries have one, ignoring case and spacing', async () => {
    expect(await paletteLine(q('c2xhcnRpYmFydGZhc3Q='))).toMatch(/\.$/)
    expect(await paletteLine(`  ${q('U2xhcnRpYmFydGZhc3Q=')} `)).toMatch(/\.$/)
    expect(await paletteLine(q('Ondx'))).toMatch(/\.$/)
  })

  test('a known first word matches with anything after it', async () => {
    expect(await paletteLine(q('c3VkbyBtb3ZlIHRhc2s='))).toMatch(/\.$/)
    expect(await paletteLine(q('c3Vkbw=='))).toBeNull()
  })
})

describe('other touches', () => {
  test('one card id has a title', () => {
    expect(cardTitle(42)).toBeTruthy()
    expect(cardTitle(41)).toBeUndefined()
  })

  test('the late line shows only in the small hours', () => {
    expect(lateLine(new Date(2026, 0, 1, 3, 0))).toBeTruthy()
    expect(lateLine(new Date(2026, 0, 1, 5, 0))).toBeNull()
    expect(lateLine(new Date(2026, 0, 1, 23, 0))).toBeNull()
  })

  test('two days of the year are different', () => {
    expect(dayFlair(new Date(2026, 2, 14)).open).toBe('3.14159')
    expect(dayFlair(new Date(2026, 4, 4)).line).toBeTruthy()
    expect(dayFlair(new Date(2026, 9, 12))).toEqual({})
  })
})
