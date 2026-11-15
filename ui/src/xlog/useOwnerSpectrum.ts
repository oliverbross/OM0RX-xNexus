// SPDX-License-Identifier: GPL-3.0-only
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { View } from './api'
import type { RxApi, RxView } from './rxApi'
import { createSpectrumReader, spectrumApi, type SpectrumBinding } from './spectrumApi'

/** Original Waterfall transport, with a separate lifecycle from radio controls.
 * A Stop intent retires pixels immediately; only a subsequent successful RX
 * observation outside the pending action can admit a new spectrum again. */
export function useOwnerSpectrum(input: {
  session: number; owner: View; receiver: RxApi; view: RxView
  pending: boolean; observation: number
}, transport = spectrumApi.read) {
  const reader = useMemo(() => createSpectrumReader({ read: transport }), [transport])
  const lifecycle = useRef({ session: input.session, receiver: input.receiver,
    generation: 0, blockedAt: null as number | null })
  const life = lifecycle.current
  if (life.session !== input.session || life.receiver !== input.receiver) {
    life.session = input.session; life.receiver = input.receiver
    life.generation++; life.blockedAt = null; reader.reset()
  }
  if (life.blockedAt != null && !input.pending && input.observation > life.blockedAt) life.blockedAt = null
  const binding: SpectrumBinding = {
    sessionGeneration: life.generation,
    owner: input.owner,
    rx: { ...input.view, enabled: input.view.enabled && !input.pending && life.blockedAt == null },
  }
  const current = useRef(binding)
  current.current = binding
  const observation = useRef(input.observation)
  observation.current = input.observation
  reader.synchronize(binding)
  const [, refresh] = useState(0)
  useEffect(() => {
    let previous = reader.inspect().resetToken
    const timer = setInterval(() => {
      const next = reader.inspect().resetToken
      if (next !== previous) { previous = next; refresh(value => value + 1) }
    }, 100)
    return () => { clearInterval(timer); reader.reset() }
  }, [reader])
  const suspend = useCallback(() => {
    lifecycle.current.generation++
    lifecycle.current.blockedAt = observation.current
    current.current = { ...current.current, sessionGeneration: lifecycle.current.generation,
      rx: { ...current.current.rx, enabled: false } }
    reader.reset(); reader.synchronize(current.current); refresh(value => value + 1)
  }, [reader])
  const getSpectrumRow = useCallback(async () => {
    const result = await reader.read(current.current)
    reader.synchronize(current.current)
    // The reader's original request-start deadline is never extended here.
    return result.kind === 'frame' && reader.isCurrent(result) ? result.spectrum : null
  }, [reader])
  return { getSpectrumRow, suspend, resetKey: `${input.session}:${reader.inspect().resetToken}` }
}
