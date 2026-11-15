// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest'
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { createAskingLogSource, type LogTransport } from './askingLogSource'
import type { AnswerTo, LogQuestion } from './logAnswers'
import {
  LogSourceProvider, __resetLogSourceForTests, setLogSource, useLatestLogAnswer,
  useLogAnswer, useLogAnswers, useLogSource, useLogStatus,
} from './logSource'

const size = { kind: 'logSize' } as const
const calls = { kind: 'workedCalls', calls: ['OM0RX'] } as const

function deferredSource() {
  const pending: { question: LogQuestion; resolve: (value: unknown) => void; reject: (error: Error) => void }[] = []
  const transport: LogTransport = <Q extends LogQuestion>(question: Q) => new Promise<AnswerTo<Q>>((resolve, reject) => {
    pending.push({ question, resolve: resolve as (value: unknown) => void, reject })
  })
  const source = createAskingLogSource(transport)
  const unsubscribes: ReturnType<typeof vi.fn>[] = []
  const releases: ReturnType<typeof vi.fn>[] = []
  const originalSubscribe = source.subscribe, originalWant = source.want
  const subscribe = vi.spyOn(source, 'subscribe').mockImplementation((listener) => {
    const release = vi.fn(originalSubscribe(listener))
    unsubscribes.push(release)
    return release
  })
  vi.spyOn(source, 'want').mockImplementation((question) => {
    const release = vi.fn(originalWant(question))
    releases.push(release)
    return release
  })
  return { source, pending, subscribe, unsubscribes, releases }
}

function Reader({ id }: { id: string }) {
  const value = useLogAnswer(size, 1)
  const status = useLogStatus(size)
  return <output data-testid={id}>{value ?? '?'}:{status?.state}:{status?.reason ?? ''}</output>
}

function Latest() {
  const value = useLatestLogAnswer(calls, 1)
  return <output data-testid="latest">{value ? value.answer.join(',') : '?'}</output>
}

function Multiple() {
  const values = useLogAnswers([size, calls], 1)
  return <output data-testid="multiple">{JSON.stringify(values)}</output>
}

afterEach(() => { cleanup(); __resetLogSourceForTests() })

describe('a cockpit log source belongs to its owner', () => {
  it('keeps the native default and two mounted owner scopes independent', async () => {
    const native = deferredSource(), left = deferredSource(), right = deferredSource()
    setLogSource(native.source)
    render(<>
      <Reader id="native" />
      <LogSourceProvider source={left.source}>
        <Reader id="left" />
        <LogSourceProvider source={right.source}><Reader id="right" /></LogSourceProvider>
      </LogSourceProvider>
    </>)
    expect([native.pending.length, left.pending.length, right.pending.length]).toEqual([1, 1, 1])
    await act(async () => {
      native.pending[0].resolve(10); left.pending[0].resolve(20); right.pending[0].resolve(30)
    })
    expect(screen.getByTestId('native').textContent).toBe('10:current:')
    expect(screen.getByTestId('left').textContent).toBe('20:current:')
    expect(screen.getByTestId('right').textContent).toBe('30:current:')
  })

  it('reports an owner failure without querying the native log', async () => {
    const native = deferredSource(), owner = deferredSource()
    setLogSource(native.source)
    render(<LogSourceProvider source={owner.source}><Reader id="owner" /></LogSourceProvider>)
    await act(async () => owner.pending[0].reject(new Error('xLog disconnected')))
    expect(screen.getByTestId('owner').textContent).toBe('?:failed:xLog disconnected')
    expect(native.pending).toHaveLength(0)
    expect(native.subscribe).not.toHaveBeenCalled()
  })

  it('a workspace switch drops a retained latest answer before the new owner replies', async () => {
    const old = deferredSource(), next = deferredSource()
    const { rerender } = render(<LogSourceProvider source={old.source}><Latest /></LogSourceProvider>)
    await act(async () => old.pending[0].resolve(['OM0RX']))
    expect(screen.getByTestId('latest').textContent).toBe('OM0RX')
    rerender(<LogSourceProvider source={next.source}><Latest /></LogSourceProvider>)
    expect(screen.getByTestId('latest').textContent).toBe('?')
    await act(async () => next.pending[0].resolve([]))
    expect(screen.getByTestId('latest').textContent).toBe('')
  })

  it('a late old-owner response cannot populate the new owner or its status', async () => {
    const old = deferredSource(), next = deferredSource()
    const { rerender } = render(<LogSourceProvider source={old.source}><Reader id="owner" /></LogSourceProvider>)
    rerender(<LogSourceProvider source={next.source}><Reader id="owner" /></LogSourceProvider>)
    expect(old.unsubscribes.length).toBeGreaterThan(0)
    expect(old.releases).toHaveLength(1)
    for (const release of [...old.unsubscribes, ...old.releases]) expect(release).toHaveBeenCalledTimes(1)
    await act(async () => old.pending[0].resolve(99))
    expect(screen.getByTestId('owner').textContent).toBe('?:asking:')
    await act(async () => old.source.refresh())
    expect(old.pending).toHaveLength(1)
    await act(async () => next.pending[0].resolve(7))
    expect(screen.getByTestId('owner').textContent).toBe('7:current:')
  })

  it('multi-question reads clear all old answers on a source change', async () => {
    const old = deferredSource(), next = deferredSource()
    const { rerender } = render(<LogSourceProvider source={old.source}><Multiple /></LogSourceProvider>)
    await act(async () => old.pending.forEach((p) => p.resolve(p.question.kind === 'logSize' ? 4 : ['OM0RX'])))
    expect(screen.getByTestId('multiple').textContent).toBe('[4,["OM0RX"]]')
    rerender(<LogSourceProvider source={next.source}><Multiple /></LogSourceProvider>)
    expect(screen.getByTestId('multiple').textContent).toBe('[null,null]')
    expect(next.pending).toHaveLength(2)
  })

  it('imperative reads use the source captured by their mounted surface', async () => {
    const native = deferredSource(), owner = deferredSource()
    setLogSource(native.source)
    function Action() {
      const source = useLogSource()
      return <button onClick={() => { void source.ask(size) }}>Read</button>
    }
    render(<LogSourceProvider source={owner.source}><Action /></LogSourceProvider>)
    fireEvent.click(screen.getByRole('button', { name: 'Read' }))
    expect(owner.pending).toHaveLength(1)
    expect(native.pending).toHaveLength(0)
    await act(async () => owner.pending[0].resolve(2))
  })
})
