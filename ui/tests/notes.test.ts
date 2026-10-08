import { describe, expect, it, vi } from 'vitest'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => null) }))

import { linkTargets } from '../src/components/NoteMarkdown'

describe('linkTargets', () => {
  it('matches the core parser: targets in order, labels dropped, no duplicates', () => {
    const md = 'See [[Bokost]] and [[#12|the push bug]].\n[[ Launch plan ]] again [[Bokost]] [[]] [[broken\n]]'
    expect(linkTargets(md)).toEqual(['Bokost', '#12', 'Launch plan'])
    expect(linkTargets('no links here [x](y)')).toEqual([])
  })
})

import { remoteOf } from '../src/api'

describe('remoteOf', () => {
  const base = { project_id: 1, path: '/code/x', auto_move: true, github_owner: null, github_repo: null, azure_org: null, azure_project: null, azure_repo: null }
  it('builds GitHub and Azure DevOps links, encoding project names', () => {
    expect(remoteOf({ ...base, github_owner: 'jhtjernsmo', github_repo: 'fjord' })).toEqual({ kind: 'github', label: 'jhtjernsmo/fjord', url: 'https://github.com/jhtjernsmo/fjord' })
    expect(remoteOf({ ...base, azure_org: 'contoso', azure_project: 'Mobile App', azure_repo: 'bokost' })?.url).toBe('https://dev.azure.com/contoso/Mobile%20App/_git/bokost')
    expect(remoteOf(base)).toBeNull()
    expect(remoteOf(null)).toBeNull()
  })
})

import { bugReportUrl, osName } from '../src/reportBug'

describe('report a bug', () => {
  it('opens a new GitHub issue with the version and OS filled in', () => {
    const url = new URL(bugReportUrl('0.2.7', 'Windows'))
    expect(url.origin + url.pathname).toBe('https://github.com/jhtjernsmo/fjord/issues/new')
    expect(url.searchParams.get('labels')).toBe('bug')
    expect(url.searchParams.get('body')).toContain('Fjord 0.2.7 on Windows')
    expect(osName('Mozilla/5.0 (Windows NT 10.0; Win64; x64)')).toBe('Windows')
    expect(osName('Mozilla/5.0 (X11; Linux x86_64)')).toBe('Linux')
  })
})
