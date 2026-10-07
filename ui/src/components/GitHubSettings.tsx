// Settings → GitHub: connect an account with a personal access token (kept in the OS
// credential store), so private repositories work without the gh CLI.
import { useEffect, useState } from 'react'
import { CircleAlert, CircleCheck } from 'lucide-react'
import { api, errorMessage } from '../api'
import type { GitHubAccount } from '../api'
import { useApp } from '../data'
import type { MessageKey } from '../i18n'

const NEW_TOKEN_URL = 'https://github.com/settings/tokens/new?scopes=repo&description=Fjord'

export function GitHubSettings() {
  const { t, toast } = useApp()
  const [account, setAccount] = useState<GitHubAccount | null | undefined>(undefined)
  const [error, setError] = useState<string | null>(null)
  const [token, setToken] = useState('')
  const [busy, setBusy] = useState(false)

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
      {account?.source !== 'env' && (
        <>
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
