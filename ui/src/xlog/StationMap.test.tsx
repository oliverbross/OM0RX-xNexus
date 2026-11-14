// @vitest-environment jsdom
import { fireEvent, render, screen } from '@testing-library/react'
import { expect, it, vi } from 'vitest'
import StationMap from './StationMap'
import type { Spot } from './api'
import contract from './fixtures/bridge-contract.json'

it('uses owner positions only and selects without tuning or resolving missing locations', () => {
  // The SDK's serde round-trip test consumes the same JSON. Do not rename wire fields here.
  const spot = contract.snapshot.spots[0] as Spot
  const select = vi.fn()
  const { container } = render(<StationMap spots={[spot, { ...spot, id: 'missing', callsign: 'K1ABC', latitude: null, longitude: null }]} selectedId={null} select={select} />)
  expect(screen.queryByRole('button', { name: 'Inspect mapped station K1ABC' })).toBeNull()
  const target = screen.getByRole('button', { name: 'Inspect mapped station OM0RX' })
  expect(target.classList.contains('xn-world-unknown')).toBe(true)
  expect(container.querySelector('.xn-world-land')?.getAttribute('d')?.length).toBeGreaterThan(1000)
  fireEvent.keyDown(target, { key: 'Enter' })
  expect(select).toHaveBeenCalledWith(spot)
})
