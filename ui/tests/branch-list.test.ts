import { describe, expect, test } from 'vitest'
import { matchBranches, visibleBranches } from '../src/branchList'

const b = (name: string, current = false) => ({ name, current })
const names = (list: { name: string }[]) => list.map((x) => x.name)
const many = Array.from({ length: 12 }, (_, i) => b(`feat/${i + 1}-thing`))

describe('branch list', () => {
  test('search matches every word, ignoring case', () => {
    const list = [b('main'), b('feat/12-Add-Login'), b('fix/7-login-crash')]
    expect(names(matchBranches(list, 'login'))).toEqual(['feat/12-Add-Login', 'fix/7-login-crash'])
    expect(names(matchBranches(list, 'fix login'))).toEqual(['fix/7-login-crash'])
    expect(names(matchBranches(list, '  '))).toEqual(['main', 'feat/12-Add-Login', 'fix/7-login-crash'])
  })

  test('shows the first eight until asked for all, and every match while searching', () => {
    expect(visibleBranches(many, '', false)).toHaveLength(8)
    expect(visibleBranches(many, '', true)).toHaveLength(12)
    expect(names(visibleBranches(many, '1', false))).toEqual(['feat/1-thing', 'feat/10-thing', 'feat/11-thing', 'feat/12-thing'])
  })

  test('the current branch stays in view in the short list', () => {
    const list = [...many.slice(0, 11), b('feat/99-mine', true)]
    const shown = visibleBranches(list, '', false)
    expect(shown).toHaveLength(8)
    expect(shown[7].name).toBe('feat/99-mine')
  })
})
