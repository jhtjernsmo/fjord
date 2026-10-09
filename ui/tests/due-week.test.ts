import { describe, expect, it } from 'vitest'
import { dueWhen, endOfWeek, localIso, parseDue } from '../src/dueWeek'

// Friday 9 October 2026, local time.
const friday = new Date(2026, 9, 9, 14, 30)

describe('due this week', () => {
  it('formats local dates', () => {
    expect(localIso(friday)).toBe('2026-10-09')
  })

  it('ends the week on Sunday', () => {
    expect(endOfWeek(friday)).toBe('2026-10-11')
    expect(endOfWeek(new Date(2026, 9, 11))).toBe('2026-10-11')
    expect(endOfWeek(new Date(2026, 9, 12))).toBe('2026-10-18')
  })

  it('crosses month ends', () => {
    expect(endOfWeek(new Date(2026, 9, 29))).toBe('2026-11-01')
  })

  it('labels each due date', () => {
    expect(dueWhen('2026-10-02', friday)).toBe('overdue')
    expect(dueWhen('2026-10-09', friday)).toBe('today')
    expect(dueWhen('2026-10-10', friday)).toBe('tomorrow')
    expect(dueWhen('2026-10-11', friday)).toBe('later')
  })

  it('parses due dates as local days', () => {
    expect(parseDue('2026-10-11').getDay()).toBe(0)
  })
})
