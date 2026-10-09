// Creating a GitHub repository for a project: the fields shared by the new-project
// dialog and the Git tab. Signing in to GitHub happens right here if needed.
import { useCallback, useEffect, useState } from 'react'
import { LogIn } from 'lucide-react'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { api, errorMessage } from '../api'
import type { GitHubAccount, NewRepo, RepoOwner } from '../api'
import { useApp } from '../data'
import { defaultParent, isValidRepoName, suggestRepoName } from '../repoName'
import { DeviceCode, useDeviceLogin } from './GitHubSettings'

/** .gitignore templates and licenses the backend accepts (see new_repo.rs). */
const GITIGNORES = ['Rust', 'Node', 'Python', 'Go', 'Java', 'Kotlin', 'Swift', 'Dart', 'C++', 'VisualStudio', 'Unity']
const LICENSES: [string, string][] = [
  ['mit', 'MIT'],
  ['apache-2.0', 'Apache 2.0'],
  ['gpl-3.0', 'GPL 3.0'],
  ['bsd-3-clause', 'BSD 3-Clause'],
  ['mpl-2.0', 'MPL 2.0'],
  ['unlicense', 'Unlicense'],
]
function joinPath(parent: string, name: string): string {
  if (!parent) return name
  const sep = parent.includes('\\') && !parent.includes('/') ? '\\' : '/'
  return parent.endsWith(sep) ? parent + name : parent + sep + name
}

/** A repository ready to create, plus the folder: the parent for a new clone, or the repo to publish. */
export interface RepoDraft {
  repo: NewRepo
  folder: string
}

interface Props {
  /** "create" makes a new repository and clones it; "publish" pushes an existing local one. */
  mode: 'create' | 'publish'
  projectName: string
  description: string
  onChange: (draft: RepoDraft | null) => void
}

export function NewRepoFields({ mode, projectName, description, onChange }: Props) {
  const { t } = useApp()
  const [account, setAccount] = useState<GitHubAccount | null | undefined>(undefined)
  const [owners, setOwners] = useState<RepoOwner[]>([])
  const [error, setError] = useState<string | null>(null)
  const [owner, setOwner] = useState('')
  const [name, setName] = useState<string | null>(null)
  const [isPrivate, setIsPrivate] = useState(true)
  const [readme, setReadme] = useState(true)
  const [gitignore, setGitignore] = useState('')
  const [license, setLicense] = useState('')
  const [folder, setFolder] = useState('')
  const onSignedIn = useCallback((a: GitHubAccount) => setAccount(a), [])
  const { device, signIn, cancel } = useDeviceLogin(onSignedIn)

  useEffect(() => {
    api
      .githubAccount()
      .then(setAccount)
      .catch(() => setAccount(null))
    if (mode === 'create') void defaultParent().then(setFolder)
  }, [mode])

  useEffect(() => {
    if (!account) return
    api
      .githubRepoOwners()
      .then((list) => {
        setOwners(list)
        setOwner((current) => current || list[0]?.login || '')
      })
      .catch((e) => setError(errorMessage(e)))
  }, [account])

  // Follows the project name until the user types their own.
  const repoName = name ?? suggestRepoName(projectName)
  const nameOk = isValidRepoName(repoName)

  useEffect(() => {
    const ready = !!account && !!owner && nameOk && !!folder
    onChange(
      ready
        ? {
            repo: {
              owner,
              name: repoName,
              description,
              private: isPrivate,
              readme: mode === 'create' && readme,
              gitignore: mode === 'create' && gitignore ? gitignore : null,
              license: mode === 'create' && license ? license : null,
            },
            folder,
          }
        : null,
    )
  }, [account, owner, repoName, nameOk, folder, description, isPrivate, readme, gitignore, license, mode, onChange])

  const choose = async () => {
    const picked = await openDialog({ directory: true, multiple: false, defaultPath: folder || undefined })
    if (typeof picked === 'string') setFolder(picked)
  }

  if (account === undefined) return <span className="hint">{t('github.checking')}</span>
  if (account === null)
    return (
      <div className="new-repo">
        <span className="hint">{t('newRepo.signInFirst')}</span>
        {device ? (
          <DeviceCode device={device} onCancel={cancel} />
        ) : (
          <button type="button" className="btn primary github-signin" onClick={signIn}>
            <LogIn size={14} /> {t('github.signIn')}
          </button>
        )}
      </div>
    )

  return (
    <div className="new-repo">
      {error && <span className="hint warn">{error}</span>}
      <div className="new-repo-name">
        <select className="input" value={owner} onChange={(e) => setOwner(e.target.value)} aria-label={t('newRepo.owner')}>
          {owners.map((o) => (
            <option key={o.login} value={o.login}>
              {o.login}
            </option>
          ))}
        </select>
        <span className="dim">/</span>
        <input
          className="input mono grow"
          value={repoName}
          onChange={(e) => setName(e.target.value)}
          aria-label={t('newRepo.name')}
          aria-invalid={!nameOk}
          spellCheck={false}
        />
      </div>
      {!nameOk && repoName && <span className="hint warn">{t('newRepo.badName')}</span>}
      <div className="segmented" role="radiogroup" aria-label={t('newRepo.visibility')}>
        <button type="button" role="radio" aria-checked={isPrivate} className={isPrivate ? 'on' : ''} onClick={() => setIsPrivate(true)}>
          {t('newRepo.private')}
        </button>
        <button type="button" role="radio" aria-checked={!isPrivate} className={!isPrivate ? 'on' : ''} onClick={() => setIsPrivate(false)}>
          {t('newRepo.public')}
        </button>
      </div>
      {mode === 'create' && (
        <div className="new-repo-files">
          <label className="toggle-label">
            <input type="checkbox" checked={readme} onChange={(e) => setReadme(e.target.checked)} />
            README
          </label>
          <select className="input" value={gitignore} onChange={(e) => setGitignore(e.target.value)} aria-label=".gitignore">
            <option value="">{t('newRepo.noGitignore')}</option>
            {GITIGNORES.map((g) => (
              <option key={g} value={g}>
                .gitignore: {g}
              </option>
            ))}
          </select>
          <select className="input" value={license} onChange={(e) => setLicense(e.target.value)} aria-label={t('newRepo.license')}>
            <option value="">{t('newRepo.noLicense')}</option>
            {LICENSES.map(([key, label]) => (
              <option key={key} value={key}>
                {label}
              </option>
            ))}
          </select>
        </div>
      )}
      <div className="new-repo-folder">
        <span className="mono path-text grow" title={mode === 'create' ? joinPath(folder, repoName) : folder}>
          {mode === 'create' ? joinPath(folder, repoName) : folder || t('newRepo.pickRepo')}
        </span>
        <button type="button" className="btn ghost" onClick={choose}>
          {t('git.choose')}
        </button>
      </div>
      <span className="hint">{mode === 'create' ? t('newRepo.createHint') : t('newRepo.publishHint')}</span>
    </div>
  )
}
