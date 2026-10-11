// Settings → Overview: which sections the Overview shows, and the weekdays you have a standup.
import { useApp } from '../data'
import { dateLocale } from '../i18n'
import type { MessageKey } from '../i18n'
import { OVERVIEW_SECTIONS, useOverviewSections } from '../overviewSections'
import { WEEK, useStandupDays } from '../standup'
import type { Weekday } from '../standup'

/** 12 October 2026 is a Monday; the weekday names come from the dates after it. */
const weekdayDate = (day: Weekday) => new Date(2026, 9, day === 0 ? 18 : 11 + day)

export function OverviewSettings() {
  const { t, locale } = useApp()
  const [sections, setSection] = useOverviewSections()
  const [days, toggleDay] = useStandupDays()
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
          </div>
        ))}
      </div>
    </div>
  )
}
