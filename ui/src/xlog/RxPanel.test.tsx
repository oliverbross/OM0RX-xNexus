// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { RxPanel } from './RxPanel'
import { rxOff, type RxApi } from './rxApi'

function native(): RxApi {
  return { status:vi.fn(async () => rxOff), start:vi.fn(async mode => ({ ...rxOff, enabled:true, mode, phase:'waiting', reason:'Waiting for xLog sharing.' })), stop:vi.fn(async () => rxOff) }
}
describe('xLog receive decoder', () => {
  it('clears a polling error when a later status request succeeds', async () => {
    const api = native()
    api.status = vi.fn().mockRejectedValueOnce(Error('Temporary status outage')).mockResolvedValue(rxOff)
    render(<RxPanel api={api} connected={true} capable={true} fresh={true} identity="owner" />)
    await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('Temporary status outage'))
    await waitFor(() => expect(api.status).toHaveBeenCalledTimes(2))
    await waitFor(() => expect(screen.queryByRole('alert')).toBeNull())
  })
  it('retains an action error through a failed poll and successful status recovery', async () => {
    const api = native()
    api.status = vi.fn().mockResolvedValueOnce(rxOff).mockRejectedValueOnce(Error('Temporary status outage')).mockResolvedValue(rxOff)
    api.start = vi.fn().mockRejectedValue(Error('Start outcome requires review'))
    render(<RxPanel api={api} connected={true} capable={true} fresh={true} identity="owner" />)
    await waitFor(() => expect(api.status).toHaveBeenCalledOnce())
    fireEvent.click(screen.getByRole('button', { name:'Start receiving' }))
    await waitFor(() => expect(screen.getByText(/Start outcome requires review/)).toBeTruthy())
    await waitFor(() => expect(screen.getByText(/Temporary status outage/)).toBeTruthy())
    expect(screen.getByText(/Start outcome requires review/)).toBeTruthy()
    await waitFor(() => expect(vi.mocked(api.status).mock.calls.length).toBeGreaterThanOrEqual(3), { timeout:2000 })
    await waitFor(() => expect(screen.queryByText(/Temporary status outage/)).toBeNull())
    expect(screen.getByText(/Start outcome requires review/)).toBeTruthy()
  })
  it('never starts capture or polls without owner capability', () => {
    const api = native()
    render(<RxPanel api={api} connected={true} capable={false} fresh={true} identity="owner" />)
    expect(screen.getByRole('button', { name:'Start receiving' })).toHaveProperty('disabled', true)
    expect(api.status).not.toHaveBeenCalled()
    expect(api.start).not.toHaveBeenCalled()
    expect(screen.getByText(/Enable RX sharing in xLog/)).toBeTruthy()
  })
  it('selects FT4 explicitly and keeps unknown RF and UTC timing visible', async () => {
    const api = native()
    render(<RxPanel api={api} connected={true} capable={true} fresh={true} identity="owner" />)
    await waitFor(() => expect(api.status).toHaveBeenCalledOnce())
    fireEvent.change(screen.getByLabelText('Receive decoder'), { target:{value:'FT4'} })
    fireEvent.click(screen.getByRole('button', { name:'Start receiving' }))
    await waitFor(() => expect(api.start).toHaveBeenCalledWith('FT4'))
    expect(screen.getByText(/UTC sync and DAX delay are unverified/)).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name:'Stop receiving' }))
    await waitFor(() => expect(api.stop).toHaveBeenCalledOnce())
  })
  it('discards a late start result after pairing changes', async () => {
    const api = native()
    let finish!: (v:typeof rxOff) => void
    api.start = vi.fn(() => new Promise<typeof rxOff>(resolve => { finish=resolve }))
    const panel = render(<RxPanel api={api} connected={true} capable={true} fresh={true} identity="old" />)
    await waitFor(() => expect(api.status).toHaveBeenCalledOnce())
    fireEvent.click(screen.getByRole('button', { name:'Start receiving' }))
    panel.rerender(<RxPanel api={api} connected={false} capable={false} fresh={false} identity="new" />)
    finish({ ...rxOff, enabled:true, reason:'OLD ROUTE RESULT' })
    await waitFor(() => expect(screen.queryByText('OLD ROUTE RESULT')).toBeNull())
    expect(screen.getByRole('button', { name:'Start receiving' })).toHaveProperty('disabled', true)
  })
  it('keeps Stop available during a pending start and ignores its late reply', async () => {
    const api = native()
    let finish!: (v:typeof rxOff) => void
    api.start = vi.fn(() => new Promise<typeof rxOff>(resolve => { finish=resolve }))
    render(<RxPanel api={api} connected={true} capable={true} fresh={true} identity="owner" />)
    await waitFor(() => expect(api.status).toHaveBeenCalledOnce())
    fireEvent.click(screen.getByRole('button', { name:'Start receiving' }))
    const stop = screen.getByRole('button', { name:'Stop receiving' })
    expect(stop).toHaveProperty('disabled', false)
    fireEvent.click(stop)
    await waitFor(() => expect(api.stop).toHaveBeenCalledOnce())
    finish({ ...rxOff, enabled:true, reason:'CANCELLED START' })
    await waitFor(() => expect(screen.queryByText('CANCELLED START')).toBeNull())
    expect(screen.getByRole('button', { name:'Start receiving' })).toHaveProperty('disabled', false)
  })
})
