// Helpers for creating GitHub repositories: names from project names ("Core UI" →
// "core-ui") and the folder new repositories are cloned into.

const LETTERS: Record<string, string> = { æ: 'ae', ø: 'o', å: 'a', ß: 'ss' }
const MAX_LENGTH = 100

/** A repository name GitHub accepts, made from a project name. */
export function suggestRepoName(projectName: string): string {
  return projectName
    .toLowerCase()
    .replace(/[æøåß]/g, (c) => LETTERS[c])
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/[^a-z0-9._-]+/g, '-')
    .replace(/-{2,}/g, '-')
    .replace(/^[-.]+|[-.]+$/g, '')
    .slice(0, MAX_LENGTH)
}

/** Same rule as the backend: letters, digits, `.`, `-` and `_`. */
export function isValidRepoName(name: string): boolean {
  return /^[A-Za-z0-9._-]{1,100}$/.test(name) && name !== '.' && name !== '..' && !name.startsWith('-')
}

const PARENT_KEY = 'fjord.repoParent'

/** Where new repositories go: the last folder used, else the home folder. */
export async function defaultParent(): Promise<string> {
  try {
    const saved = localStorage.getItem(PARENT_KEY)
    if (saved) return saved
  } catch {
    /* storage unavailable */
  }
  try {
    const { homeDir } = await import('@tauri-apps/api/path')
    return await homeDir()
  } catch {
    return ''
  }
}

export function rememberParent(folder: string): void {
  try {
    localStorage.setItem(PARENT_KEY, folder)
  } catch {
    /* storage unavailable */
  }
}
