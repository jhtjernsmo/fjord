// Settings window: a menu of sections on the left, the section on the right.
import { useEffect, useMemo, useRef, useState } from 'react'
import { Bot, Database, Keyboard, Palette, PlugZap, RefreshCw, Search, SlidersHorizontal, UserRound, X } from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { api } from '../api'
import { useApp } from '../data'
import { LOCALES } from '../i18n'
import type { MessageKey } from '../i18n'
import { AzureImportSettings } from './AzureImport'
import { AzureSettings } from './AzureSettings'
import { UpdateSettings } from './Dialogs'
import { GitHubSettings } from './GitHubSettings'
import { ThemeSettings } from './ThemeSettings'

type SectionId = 'general' | 'appearance' | 'accounts' | 'integrations' | 'keyboard' | 'updates' | 'agents' | 'data'

interface Section {
  id: SectionId
  icon: LucideIcon
  /** Extra words the search box matches, besides the section title. */
  keywords: string
}

const SECTIONS: Section[] = [
  { id: 'general', icon: SlidersHorizontal, keywords: 'name user language norsk english' },
  { id: 'appearance', icon: Palette, keywords: 'theme dark light color colour font size density accent tema farge' },
  { id: 'accounts', icon: UserRound, keywords: 'github azure devops token login sign in az' },
  { id: 'integrations', icon: PlugZap, keywords: 'azure boards import work items' },
  { id: 'keyboard', icon: Keyboard, keywords: 'shortcuts keys keymap hurtigtaster' },
  { id: 'updates', icon: RefreshCw, keywords: 'update version release oppdatering' },
  { id: 'agents', icon: Bot, keywords: 'mcp ai agent claude' },
  { id: 'data', icon: Database, keywords: 'folder backup database path' },
]

const TAB_KEY = 'fjord.settingsTab'

function loadTab(): SectionId {
  try {
    const v = localStorage.getItem(TAB_KEY)
    return SECTIONS.some((s) => s.id === v) ? (v as SectionId) : 'general'
  } catch {
    return 'general'
  }
}

function GeneralSection() {
  const { t, locale, setLocale, run, toast, refresh } = useApp()
  const [userName, setUserName] = useState('')
  const [savedName, setSavedName] = useState('')
  const [rewrite, setRewrite] = useState(true)
  useEffect(() => {
    api.actor().then((a) => {
      setUserName(a)
      setSavedName(a)
    })
  }, [])
  const saveName = async () => {
    const name = await run(api.renameUser(userName, rewrite))
    if (name) {
      setSavedName(name)
      toast(t('settings.saved'), 'success')
      refresh()
    }
  }
  return (
    <>
      <div className="row">
        {t('settings.user')}
        <form
          className="user-row"
          onSubmit={(e) => {
            e.preventDefault()
            if (userName.trim() && userName.trim() !== savedName) saveName()
          }}
        >
          <input className="input grow" value={userName} onChange={(e) => setUserName(e.target.value)} aria-label={t('settings.user')} />
          <button className="btn primary" disabled={!userName.trim() || userName.trim() === savedName}>
            {t('settings.save')}
          </button>
        </form>
        <label className="toggle-label">
          <input type="checkbox" checked={rewrite} onChange={(e) => setRewrite(e.target.checked)} />
          {t('settings.rewrite')}
        </label>
        <span className="hint">{t('settings.userHint')}</span>
      </div>
      <div className="row">
        {t('settings.language')}
        <div className="segmented">
          {LOCALES.map((l) => (
            <button key={l.id} className={locale === l.id ? 'on' : ''} onClick={() => setLocale(l.id)}>
              {l.label}
            </button>
          ))}
        </div>
      </div>
    </>
  )
}

function PathsSection({ kind }: { kind: 'keyboard' | 'agents' | 'data' }) {
  const { t } = useApp()
  const [paths, setPaths] = useState<{ data: string; keymap: string } | null>(null)
  useEffect(() => {
    api.dataPaths().then(setPaths).catch(() => setPaths(null))
  }, [])
  if (kind === 'agents')
    return (
      <div className="row">
        {t('settings.agents')}
        <span className="hint mono">{t('settings.agentsHint')}</span>
      </div>
    )
  return kind === 'keyboard' ? (
    <div className="row">
      {t('settings.keys')}
      <span className="hint">{t('settings.keysHint')}</span>
      {paths && <code className="path">{paths.keymap}</code>}
    </div>
  ) : (
    <div className="row">
      {t('settings.data')}
      <span className="hint">{t('settings.dataHint')}</span>
      {paths && <code className="path">{paths.data}</code>}
    </div>
  )
}

function SectionBody({ id }: { id: SectionId }) {
  switch (id) {
    case 'general':
      return <GeneralSection />
    case 'appearance':
      return <ThemeSettings />
    case 'accounts':
      return (
        <>
          <GitHubSettings />
          <AzureSettings />
        </>
      )
    case 'integrations':
      return <AzureImportSettings />
    case 'updates':
      return <UpdateSettings />
    default:
      return <PathsSection kind={id} />
  }
}

export function SettingsWindow({ onClose }: { onClose: () => void }) {
  const { t } = useApp()
  const [tab, setTab] = useState<SectionId>(loadTab)
  const [query, setQuery] = useState('')
  const searchRef = useRef<HTMLInputElement>(null)

  const title = (id: SectionId) => t(`settings.section.${id}` as MessageKey)
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase()
    return q ? SECTIONS.filter((s) => `${title(s.id)} ${s.keywords}`.toLowerCase().includes(q)) : SECTIONS
  }, [query]) // eslint-disable-line react-hooks/exhaustive-deps

  const open = (id: SectionId) => {
    setTab(id)
    try {
      localStorage.setItem(TAB_KEY, id)
    } catch {
      /* non-fatal */
    }
  }

  // Searching jumps to the first match.
  useEffect(() => {
    if (shown.length > 0 && !shown.some((s) => s.id === tab)) open(shown[0].id)
  }, [shown]) // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div className="overlay" onMouseDown={onClose}>
      <div className="settings-window" onMouseDown={(e) => e.stopPropagation()} role="dialog" aria-label={t('settings.title')}>
        <nav className="settings-nav" aria-label={t('settings.title')}>
          <h2>{t('settings.title')}</h2>
          <label className="filter-search small-search">
            <Search size={13} />
            <input
              ref={searchRef}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t('settings.search')}
              aria-label={t('settings.search')}
              onKeyDown={(e) => e.key === 'Escape' && (query ? (e.stopPropagation(), setQuery('')) : undefined)}
            />
          </label>
          {shown.map((s) => (
            <button key={s.id} className={`nav-item ${tab === s.id ? 'active' : ''}`} onClick={() => open(s.id)}>
              <s.icon size={15} strokeWidth={1.7} />
              <span className="name">{title(s.id)}</span>
            </button>
          ))}
          {shown.length === 0 && <span className="hint">{t('settings.noMatch')}</span>}
        </nav>
        <section className="settings-body">
          <header>
            <h3>{title(tab)}</h3>
            <button className="icon-btn" onClick={onClose} aria-label={t('task.close')}>
              <X size={16} />
            </button>
          </header>
          <div className="settings-content">
            <SectionBody id={tab} />
          </div>
        </section>
      </div>
    </div>
  )
}
