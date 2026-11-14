// @vitest-environment jsdom
import { fireEvent, render, screen } from '@testing-library/react'
import { expect, it, vi } from 'vitest'
import StationMap from './StationMap'
import type { Spot } from './api'

it('uses owner positions only and selects without tuning or resolving missing locations', () => {
  const spot: Spot = { id: 'a', callsign: 'OM0RX', frequency_hz: 14_025_000, mode: 'CW', observed_at_unix_ms: Date.now(), source: 'owner', lat: 48.15, lon: 17.11, country: 'Slovak Republic', need: { status: 'unknown', score: 0, reasons: [] } }
  const select = vi.fn()
  const { container } = render(<StationMap spots={[spot, { ...spot, id: 'missing', callsign: 'K1ABC', lat: null, lon: null }]} selectedId={null} select={select} />)
  expect(screen.queryByRole('button', { name: 'Inspect mapped station K1ABC' })).toBeNull()
  const target = screen.getByRole('button', { name: 'Inspect mapped station OM0RX' })
  expect(target.classList.contains('xn-world-unknown')).toBe(true)
  expect(container.querySelector('.xn-world-land')?.getAttribute('d')?.length).toBeGreaterThan(1000)
  fireEvent.keyDown(target, { key: 'Enter' })
  expect(select).toHaveBeenCalledWith(spot)
})
