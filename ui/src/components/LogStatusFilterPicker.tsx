// SPDX-License-Identifier: GPL-3.0-only
// OM0RX-xNexus: the "Status" chip — a popover with OM0RX-xLog's two filter boxes, Call status and
// DXCC status, each a set of tick boxes (ticked = shown), plus one-click presets. Portaled like
// CountryExcludePicker so the chip bar's clip cannot swallow it in a narrow pane. The status names
// are xLog's own vocabulary (the codes the CS / DS columns show), so they are not translated.
import * as RM from '@radix-ui/react-dropdown-menu'
import { MenuContent, MenuRoot } from './ui/MenuParts'
import { t } from '../i18n'
import { logStatusCode, logStatusColor } from './LogStatusCell'
import {
  CS_VALUES,
  DS_VALUES,
  LOG_STATUS_PRESETS,
  hiddenCount,
  matchingPreset,
  toggleStatus,
  type LogStatusFilter,
  type LogStatusPreset,
} from '../logStatusFilter'

const PRESET_ORDER: LogStatusPreset[] = ['all', 'newCalls', 'newDxcc', 'unconfirmed']

export function LogStatusFilterPicker({
  filter,
  onChange,
  className = 'od-chip',
}: {
  filter: LogStatusFilter
  onChange: (f: LogStatusFilter) => void
  /** The trigger's chip class: Band Activity's `od-chip`, or the roster's own. */
  className?: string
}) {
  const count = hiddenCount(filter)
  const preset = matchingPreset(filter)
  const item = (kind: 'cs' | 'ds', status: string) => {
    const hidden = (kind === 'cs' ? filter.hideCs : filter.hideDs).includes(status)
    return (
      <RM.CheckboxItem
        key={`${kind}-${status}`}
        className="ui-menu-item country-item log-status-item"
        checked={!hidden}
        // Ticking must not close the menu: the operator is choosing a SET.
        onSelect={(e) => e.preventDefault()}
        onCheckedChange={() => onChange(toggleStatus(filter, kind, status))}
      >
        <span className="log-status-code" style={{ color: logStatusColor(status) }} aria-hidden="true">
          {status === 'Unknown' ? '?' : logStatusCode(status)}
        </span>
        {status}
      </RM.CheckboxItem>
    )
  }
  return (
    <MenuRoot>
      <RM.Trigger asChild>
        <button
          type="button"
          className={`${className} od-logstatus${count > 0 ? ' active' : ''}`}
          title={t('logStatusFilter.chip.title')}
          data-testid="log-status-filter"
        >
          {t('logStatusFilter.chip.label')}
          {count > 0 ? ` · ${count}` : ''}
        </button>
      </RM.Trigger>
      <RM.Portal>
        <MenuContent className="ui-menu country-menu log-status-menu" sideOffset={4} align="end" collisionPadding={8}>
          <div style={{ zoom: 'var(--ui-zoom, 1)' }}>
            <div className="country-menu-head">{t('logStatusFilter.presets')}</div>
            {PRESET_ORDER.map((p) => (
              <RM.CheckboxItem
                key={`preset-${p}`}
                className="ui-menu-item country-item"
                checked={preset === p}
                onSelect={(e) => e.preventDefault()}
                onCheckedChange={() => onChange({ ...LOG_STATUS_PRESETS[p] })}
              >
                {t(`logStatusFilter.preset.${p}`)}
              </RM.CheckboxItem>
            ))}
            <div className="log-status-boxes">
              <div className="log-status-box" role="group" aria-label={t('logStatusFilter.cs')}>
                <div className="country-menu-head">{t('logStatusFilter.cs')}</div>
                {CS_VALUES.map((s) => item('cs', s))}
              </div>
              <div className="log-status-box" role="group" aria-label={t('logStatusFilter.ds')}>
                <div className="country-menu-head">{t('logStatusFilter.ds')}</div>
                {DS_VALUES.map((s) => item('ds', s))}
              </div>
            </div>
            <div className="country-menu-note">{t('logStatusFilter.note')}</div>
          </div>
        </MenuContent>
      </RM.Portal>
    </MenuRoot>
  )
}
