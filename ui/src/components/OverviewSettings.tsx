// Settings → Overview: which sections the Overview shows, and the weekdays you have a standup.
import { useApp, useLive } from '../data'
import { api } from '../api'
import { dateLocale } from '../i18n'
import type { MessageKey } from '../i18n'
import { OVERVIEW_SECTIONS, useOverviewSections } from '../overviewSections'
import { WEEK, useStandupDays, useStandupProject } from '../standup'
import type { Weekday } from '../standup'

/** 12 October 2026 is a Monday; the weekday names come from the dates after it. */
const weekdayDate = (day: Weekday) => new Date(2026, 9, day === 0 ? 18 : 11 + day)

export function OverviewSettings() {
  const { t, locale } = useApp()
  const [sections, setSection] = useOverviewSections()
  const [days, toggleDay] = useStandupDays()
  const [standupProject, setStandupProject] = useStandupProject()
  const [projects] = useLive(() => api.listProjects(false), [])
  return (
    <div className="row">
      {t('overviewSettings.title')}
      <span className="hint">{t('overviewSettings.hint')}</span>
      <div className="overview-toggles">
        {OVERVIEW_SECTIONS.map((id) => (
          <div key={id} className="overview-toggle">
            <label className="toggle-label">
              <input type="checkbox" checked={sections[id]} onChange={(e) => setSection(id, e.target.checked)} />
              <span className="overview-toggle-name">{t(`overviewSettings.${id}` as MessageKey)}</span>
              {(id === 'standup' || id === 'waiting') && <span className="beta">beta</span>}
            </label>
            <span className="hint">{t(`overviewSettings.${id}Hint` as MessageKey)}</span>
            {id === 'standup' && sections.standup && (
              <div className="weekday-picker" role="group" aria-label={t('overviewSettings.standupDays')}>
                {WEEK.map((day) => (
                  <button key={day} className={days.includes(day) ? 'on' : ''} aria-pressed={days.includes(day)} onClick={() => toggleDay(day)}>
                    {weekdayDate(day).toLocaleDateString(dateLocale(locale), { weekday: 'short' })}
                  </button>
                ))}
              </div>
            )}
            {id === 'standup' && sections.standup && (
              <label className="standup-project-pick">
                {t('overviewSettings.standupProject')}
                <select className="input" value={standupProject ?? ''} onChange={(e) => setStandupProject(e.target.value ? Number(e.target.value) : null)}>
                  <option value="">{t('overviewSettings.allProjects')}</option>
                  {(projects ?? []).map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </select>
                <span className="hint">{t('overviewSettings.standupProjectHint')}</span>
              </label>
            )}
          </div>
        ))}
      </div>
    </div>
  )
}
