// Settings → Azure DevOps (beta): one personal access token per organization, kept
// in the OS credential store. Organization names (not secret) are remembered locally
// so their status can be shown.
import { useEffect, useState } from 'react'
import { CircleCheck, CircleDashed } from 'lucide-react'
import { api, errorMessage } from '../api'
import type { AzureAccount } from '../api'
import { useApp } from '../data'
import type { MessageKey } from '../i18n'

const ORGS_KEY = 'fjord.azureOrgs'

function loadOrgs(): string[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(ORGS_KEY) ?? '[]')
    return Array.isArray(parsed) ? parsed.filter((o): o is string => typeof o === 'string') : []
  } catch {
    return []
  }
}

function saveOrgs(orgs: string[]): void {
  try {
    localStorage.setItem(ORGS_KEY, JSON.stringify(orgs))
  } catch {
    /* non-fatal */
  }
}

function tokenUrl(org: string): string {
  return `https://dev.azure.com/${encodeURIComponent(org)}/_usersSettings/tokens`
}

function OrgStatus({ org, onRemoved }: { org: string; onRemoved: () => void }) {
  const { t, toast } = useApp()
  const [account, setAccount] = useState<AzureAccount | null | undefined>(undefined)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    api
      .azureAccount(org)
      .then(setAccount)
      .catch((e) => {
        setAccount(null)
        setError(errorMessage(e))
      })
  }, [org])

  const disconnect = async () => {
    try {
      await api.disconnectAzure(org)
      onRemoved()
    } catch (e) {
      toast(errorMessage(e), 'error')
    }
  }

  return (
    <div className="github-account">
      {account ? <CircleCheck size={15} className="ok" /> : <CircleDashed size={15} className="dim" />}
      <span className="mono">{org}</span>
      <span className="dim">
        {account === undefined
          ? t('github.checking')
          : account
            ? `${account.user} · ${t(`azure.source.${account.source}` as MessageKey)}`
            : (error ?? t('azure.notConnected'))}
      </span>
      <button className="btn ghost" onClick={disconnect}>
        {t('github.disconnect')}
      </button>
    </div>
  )
}

export function AzureSettings() {
  const { t, toast } = useApp()
  const [orgs, setOrgs] = useState(loadOrgs)
  const [org, setOrg] = useState('')
  const [token, setToken] = useState('')
  const [busy, setBusy] = useState(false)

  const update = (next: string[]) => {
    setOrgs(next)
    saveOrgs(next)
  }

  const connect = async () => {
    setBusy(true)
    try {
      // No token: use the Azure CLI's sign-in (az login).
      const account = token.trim() ? await api.connectAzure(org.trim(), token) : await api.connectAzureCli(org.trim())
      update([...orgs.filter((o) => o.toLowerCase() !== account.org.toLowerCase()), account.org])
      setOrg('')
      setToken('')
      toast(t('azure.connected', { org: account.org, user: account.user }), 'success')
    } catch (e) {
      toast(errorMessage(e), 'error')
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="row">
      <span>
        Azure DevOps <span className="beta">beta</span>
      </span>
      {orgs.map((o) => (
        <OrgStatus key={o} org={o} onRemoved={() => update(orgs.filter((x) => x !== o))} />
      ))}
      <div className="user-row">
        <input
          className="input mono"
          value={org}
          placeholder={t('azure.org')}
          spellCheck={false}
          onChange={(e) => setOrg(e.target.value)}
          aria-label={t('azure.org')}
        />
        <input
          className="input mono grow"
          type="password"
          autoComplete="off"
          spellCheck={false}
          value={token}
          placeholder={t('azure.token')}
          onChange={(e) => setToken(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && org.trim() && connect()}
          aria-label={t('azure.token')}
        />
        <button className="btn primary" disabled={busy || !org.trim()} onClick={connect}>
          {token.trim() ? t('github.connect') : t('azure.useCli')}
        </button>
      </div>
      <span className="hint">
        {t('azure.hint')}{' '}
        {org.trim() && (
          <a href={tokenUrl(org.trim())} onClick={(e) => (e.preventDefault(), api.openUrl(tokenUrl(org.trim())))}>
            {t('azure.createToken')}
          </a>
        )}
      </span>
    </div>
  )
}
