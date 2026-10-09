// In-app updates: checks GitHub for a signed new version and installs it on request.
import { useCallback, useEffect, useState } from 'react'
import { check } from '@tauri-apps/plugin-updater'
import type { Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { Download, RefreshCw, X } from 'lucide-react'
import { api, errorMessage } from '../api'
import { useApp } from '../data'

const CHECK_KEY = 'fjord.updateCheck'
const STARTUP_DELAY_MS = 4000

export function updateCheckEnabled(): boolean {
  try {
    return localStorage.getItem(CHECK_KEY) !== 'off'
  } catch {
    return true
  }
}

export function setUpdateCheckEnabled(on: boolean): void {
  try {
    localStorage.setItem(CHECK_KEY, on ? 'on' : 'off')
  } catch {
    /* non-fatal */
  }
}

/** The installed app version, e.g. "0.2.7" (null in the browser preview). */
export async function appVersion(): Promise<string | null> {
  try {
    const { getVersion } = await import('@tauri-apps/api/app')
    return await getVersion()
  } catch {
    return null
  }
}

let storeInstall: Promise<boolean> | null = null
/** The Microsoft Store version is updated by the Store, never by Fjord itself. */
export function installedFromStore(): Promise<boolean> {
  storeInstall ??= api.installedFromStore().catch(() => false)
  return storeInstall
}

/** Shared by the banner and Settings → "Check for updates". */
export function useUpdater() {
  const { t, toast } = useApp()
  const [update, setUpdate] = useState<Update | null>(null)
  const [progress, setProgress] = useState<number | null>(null)
  const [checking, setChecking] = useState(false)

  const runCheck = useCallback(
    async (manual: boolean) => {
      setChecking(true)
      try {
        const found = await check()
        setUpdate(found)
        if (!found && manual) {
          const version = await appVersion()
          toast(version ? `${t('update.upToDate')} (${version})` : t('update.upToDate'), 'success')
        }
      } catch (err) {
        // Dev builds and offline machines end up here; only bother the user if they asked.
        if (manual) toast(`${t('update.failed')}: ${errorMessage(err)}`, 'error')
      } finally {
        setChecking(false)
      }
    },
    [t, toast],
  )

  const install = useCallback(async () => {
    if (!update) return
    let total = 0
    let done = 0
    setProgress(0)
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === 'Started') total = event.data.contentLength ?? 0
        else if (event.event === 'Progress') {
          done += event.data.chunkLength
          setProgress(total ? Math.round((done / total) * 100) : null)
        }
      })
      await relaunch()
    } catch (err) {
      setProgress(null)
      toast(`${t('update.failed')}: ${errorMessage(err)}`, 'error')
    }
  }, [update, t, toast])

  return { update, progress, checking, runCheck, install, dismiss: () => setUpdate(null) }
}

export function UpdateBanner() {
  const { t } = useApp()
  const { update, progress, runCheck, install, dismiss } = useUpdater()

  useEffect(() => {
    if (!updateCheckEnabled()) return
    const id = window.setTimeout(() => {
      void installedFromStore().then((store) => {
        if (!store) void runCheck(false)
      })
    }, STARTUP_DELAY_MS)
    return () => window.clearTimeout(id)
  }, [runCheck])

  if (!update) return null
  return (
    <div className="update-banner" role="status">
      <Download size={15} />
      <span className="grow">{t('update.available', { version: update.version })}</span>
      {progress !== null ? (
        <span className="mono dim">
          <RefreshCw size={12} className="spin" /> {t('update.installing')} {progress ? `${progress}%` : ''}
        </span>
      ) : (
        <>
          <button className="btn primary" onClick={install}>
            {t('update.install')}
          </button>
          <button className="icon-btn" onClick={dismiss} aria-label={t('task.close')}>
            <X size={14} />
          </button>
        </>
      )}
    </div>
  )
}
