// Settings → GitHub: connect an account with a personal access token (kept in the OS
// credential store), so private repositories work without the gh CLI.
import { useEffect, useState } from 'react'
import { CircleAlert, CircleCheck, Copy, LogIn } from 'lucide-react'
import { api, errorMessage } from '../api'
import type { DeviceLogin, GitHubAccount } from '../api'
import { useApp } from '../data'
import type { MessageKey } from '../i18n'

const NEW_TOKEN_URL = 'https://github.com/settings/tokens/new?scopes=repo&description=Fjord'

export function GitHubSettings() {
  const { t, toast } = useApp()
  const [account, setAccount] = useState<GitHubAccount | null | undefined>(undefined)
  const [error, setError] = useState<string | null>(null)
  const [token, setToken] = useState('')
  const [busy, setBusy] = useState(false)
  const [device, setDevice] = useState<DeviceLogin | null>(null)

  // While a browser sign-in is open, poll GitHub at the interval it asked for.
  useEffect(() => {
    if (!device) return
    let interval = device.interval
    let timer = 0
    const deadline = Date.now() + device.expires_in * 1000
    const tick = async () => {
      try {
        const poll = await api.pollGithubLogin()
        if (poll.status === 'done') {
          setAccount(poll.account)
          setDevice(null)
          toast(t('github.connected', { login: poll.account.login }), 'success')
          return
        }
        if (poll.status === 'expired' || poll.status === 'denied' || Date.now() > deadline) {
          setDevice(null)
          toast(t(poll.status === 'denied' ? 'github.loginDenied' : 'github.loginExpired'), 'error')
          return
        }
        if (poll.status === 'slow_down') interval = poll.interval
      } catch (e) {
        setDevice(null)
        toast(errorMessage(e), 'error')
        return
      }
      timer = window.setTimeout(tick, interval * 1000)
    }
    timer = window.setTimeout(tick, interval * 1000)
    return () => window.clearTimeout(timer)
  }, [device, t, toast])

  const signIn = async () => {
    try {
      const login = await api.startGithubLogin()
      setDevice(login)
      navigator.clipboard?.writeText(login.user_code).catch(() => undefined)
      api.openUrl(login.verification_uri).catch(() => undefined)
    } catch (e) {
      toast(errorMessage(e), 'error')
    }
  }

  const load = () => {
    setError(null)
    api
      .githubAccount()
      .then(setAccount)
      .catch((e) => {
        setAccount(null)
        setError(errorMessage(e))
      })
  }
  useEffect(load, [])

  const connect = async () => {
    setBusy(true)
    try {
      const a = await api.connectGithub(token)
      setAccount(a)
      setToken('')
      setError(null)
      toast(t('github.connected', { login: a.login }), 'success')
    } catch (e) {
      toast(errorMessage(e), 'error')
    } finally {
      setBusy(false)
    }
  }

  const disconnect = async () => {
    setBusy(true)
    try {
      await api.disconnectGithub()
      load()
    } catch (e) {
      toast(errorMessage(e), 'error')
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="row">
      GitHub
      {account === undefined && <span className="hint">{t('github.checking')}</span>}
      {account && (
        <div className="github-account">
          <CircleCheck size={15} className="ok" />
          <span>
            {t('github.signedIn', { login: account.login })} <span className="dim">· {t(`github.source.${account.source}` as MessageKey)}</span>
          </span>
          {account.source === 'saved' && (
            <button className="btn" disabled={busy} onClick={disconnect}>
              {t('github.disconnect')}
            </button>
          )}
        </div>
      )}
      {account && !account.private_repos && (
        <span className="hint warn">
          <CircleAlert size={13} /> {t('github.noRepoScope')}
        </span>
      )}
      {error && <span className="hint warn">{error}</span>}
      {device && (
        <div className="device-login">
          <span className="hint">{t('github.enterCode')}</span>
          <div className="device-code">
            <span className="mono">{device.user_code}</span>
            <button className="icon-btn" onClick={() => navigator.clipboard?.writeText(device.user_code)} aria-label={t('github.copyCode')} title={t('github.copyCode')}>
              <Copy size={14} />
            </button>
          </div>
          <span className="hint">
            <a href={device.verification_uri} onClick={(e) => (e.preventDefault(), api.openUrl(device.verification_uri))}>
              {device.verification_uri}
            </a>{' '}
            · {t('github.waiting')}
          </span>
          <button className="btn ghost" onClick={() => setDevice(null)}>
            {t('dialog.cancel')}
          </button>
        </div>
      )}
      {account?.source !== 'env' && !device && (
        <button className="btn primary github-signin" onClick={signIn}>
          <LogIn size={14} /> {account ? t('github.signInAgain') : t('github.signIn')}
        </button>
      )}
      {account?.source !== 'env' && (
        <>
          <span className="hint">{t('github.orToken')}</span>
          <div className="user-row">
            <input
              className="input mono grow"
              type="password"
              autoComplete="off"
              spellCheck={false}
              value={token}
              placeholder={account?.source === 'saved' ? t('github.replaceToken') : 'ghp_… / github_pat_…'}
              onChange={(e) => setToken(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && token.trim() && connect()}
              aria-label={t('github.token')}
            />
            <button className="btn primary" disabled={busy || !token.trim()} onClick={connect}>
              {t('github.connect')}
            </button>
          </div>
          <span className="hint">
            {t('github.hint')}{' '}
            <a href={NEW_TOKEN_URL} onClick={(e) => (e.preventDefault(), api.openUrl(NEW_TOKEN_URL))}>
              {t('github.createToken')}
            </a>
          </span>
        </>
      )}
    </div>
  )
}
