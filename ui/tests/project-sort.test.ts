import { describe, expect, test } from 'vitest'
import type { ProjectSummary } from '../src/api'
import { leadingNumber, sortProjects } from '../src/projectSort'

const project = (id: number, name: string, created_at = '2026-01-01T00:00:00Z') =>
  ({ id, name, created_at }) as ProjectSummary
const names = (list: ProjectSummary[]) => list.map((p) => p.name)

const projects = [
  project(1, 'Bokost', '2026-03-01T00:00:00Z'),
  project(2, '10 Archive', '2026-01-01T00:00:00Z'),
  project(3, 'agent', '2026-02-01T00:00:00Z'),
  project(4, '2 Fjord', '2026-04-01T00:00:00Z'),
  project(5, '1 Work', '2026-05-01T00:00:00Z'),
]

describe('project sorting', () => {
  test('reads the number a name starts with', () => {
    expect(leadingNumber('12 Bokost')).toBe(12)
    expect(leadingNumber('  3-fjord')).toBe(3)
    expect(leadingNumber('Fjord 2')).toBeNull()
  })

  test('numbered puts names starting with a number first, by value, then the rest A–Z', () => {
    expect(names(sortProjects(projects, { by: 'numbered', reversed: false }))).toEqual(['1 Work', '2 Fjord', '10 Archive', 'agent', 'Bokost'])
  })

  test('alphabetical ignores case and compares numbers naturally', () => {
    expect(names(sortProjects(projects, { by: 'alphabetical', reversed: false }))).toEqual(['1 Work', '2 Fjord', '10 Archive', 'agent', 'Bokost'])
    expect(names(sortProjects([project(1, 'b'), project(2, 'A'), project(3, 'c')], { by: 'alphabetical', reversed: false }))).toEqual(['A', 'b', 'c'])
  })

  test('reversed flips the order', () => {
    expect(names(sortProjects(projects, { by: 'numbered', reversed: true }))).toEqual(['Bokost', 'agent', '10 Archive', '2 Fjord', '1 Work'])
  })

  test('last used puts the most recently opened first and the rest in numbered order', () => {
    const used = { 1: 2000, 3: 3000 }
    expect(names(sortProjects(projects, { by: 'recent', reversed: false }, used))).toEqual(['agent', 'Bokost', '1 Work', '2 Fjord', '10 Archive'])
  })

  test('created puts the newest first', () => {
    expect(names(sortProjects(projects, { by: 'created', reversed: false }))).toEqual(['1 Work', '2 Fjord', 'Bokost', 'agent', '10 Archive'])
  })

  test('does not change the list it was given', () => {
    const before = names(projects)
    sortProjects(projects, { by: 'alphabetical', reversed: true })
    expect(names(projects)).toEqual(before)
  })
})
