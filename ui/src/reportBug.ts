// "Report a bug": opens a new GitHub issue with the version and OS filled in.
import { api } from './api'

const NEW_ISSUE = 'https://github.com/jhtjernsmo/fjord/issues/new'

/** Best-effort OS name from the user agent (good enough for a bug report). */
export function osName(ua: string = navigator.userAgent): string {
  if (/Windows/i.test(ua)) return 'Windows'
  if (/Mac/i.test(ua)) return 'macOS'
  if (/Linux|X11/i.test(ua)) return 'Linux'
  return 'unknown OS'
}

export function bugReportUrl(version: string, os: string): string {
  const body = [
    '**What happened?**',
    '',
    '',
    '**What did you expect to happen?**',
    '',
    '',
    '**Steps to reproduce**',
    '1. ',
    '',
    '---',
    `Fjord ${version} on ${os}`,
  ].join('\n')
  const params = new URLSearchParams({ labels: 'bug', body })
  return `${NEW_ISSUE}?${params.toString()}`
}

export async function reportBug(): Promise<void> {
  let version = 'unknown version'
  try {
    const { getVersion } = await import('@tauri-apps/api/app')
    version = await getVersion()
  } catch {
    /* outside the app (tests, browser) */
  }
  await api.openUrl(bugReportUrl(version, osName()))
}
