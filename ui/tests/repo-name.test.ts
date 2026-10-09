import { describe, expect, test } from 'vitest'
import { isValidRepoName, suggestRepoName } from '../src/repoName'

describe('suggestRepoName', () => {
  test('turns a project name into a lowercase, dashed repository name', () => {
    expect(suggestRepoName('Core UI')).toBe('core-ui')
    expect(suggestRepoName('  My   App!! ')).toBe('my-app')
  })

  test('spells out Norwegian and accented letters', () => {
    expect(suggestRepoName('Møteutvalg')).toBe('moteutvalg')
    expect(suggestRepoName('Blåbær café')).toBe('blabaer-cafe')
  })

  test('keeps dots and underscores but trims them from the ends', () => {
    expect(suggestRepoName('.dotfiles_v2.')).toBe('dotfiles_v2')
  })

  test('always gives a name the backend accepts, or an empty one', () => {
    for (const name of ['Fjord', '10 Archive', '日本語', '---']) {
      const repo = suggestRepoName(name)
      expect(repo === '' || isValidRepoName(repo)).toBe(true)
    }
  })
})

describe('isValidRepoName', () => {
  test('rejects names that could escape the folder or read as options', () => {
    for (const name of ['', '.', '..', '../x', 'a/b', '-x', 'has space']) expect(isValidRepoName(name)).toBe(false)
  })
})
