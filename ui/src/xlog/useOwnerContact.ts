// SPDX-License-Identifier: GPL-3.0-only
import { useEffect, useMemo, useRef, useState } from 'react'
import { confirmDialog } from '../confirm'
import { t } from '../i18n'
import { createOwnerContactEditor, type OwnerContactEditor, type OwnerEditorSource } from '../features/ownerContactEditor'
import type { OwnerContactRead } from '../features/ownerContactDetail'
import { contactReads, type Bridge } from './api'
import { createOwnerContactReader } from './ownerContactReader'
import type { OwnerReadBinding } from './ownerHistoryReader'
import { canonicalContactId } from './ownerHistoryPresentation'
import { loadOwnerEditorDraft } from './draftStore'
import type { OwnerConnectionScope, useOwnerConnection } from './useOwnerConnection'

type Connection = Pick<ReturnType<typeof useOwnerConnection>, 'view'|'session'|'isCurrentReadConnection'>
type Selection = { id: string; key: string; editor: OwnerContactEditor }
const scopeOf = (connection: Connection): OwnerConnectionScope | null => {
  const { view, session } = connection
  return view.connection_token && view.hello ? { session, connectionToken: view.connection_token,
    serverInstance: view.hello.server_instance, workspaceId: view.hello.workspace_id } : null
}

/** Explicit exact-ID opens, followed only by successful visible read refreshes.
 * Draft ownership is outside the card's presentation/remount lifecycle. */
export function useOwnerContact(connection: Connection, api: Bridge,
  canOpenRow: (call: string, id: string | null | undefined) => boolean, transport = contactReads) {
  const reader = useMemo(() => createOwnerContactReader({ transport }), [transport])
  const [, changed] = useState(0)
  const [saved, setSaved] = useState(loadOwnerEditorDraft)
  const selected = useRef<Selection | null>(null)
  const serial = useRef(0)
  const current = useRef({ connection, api, canOpenRow, reader })
  current.current = { connection, api, canOpenRow, reader }
  const binding: OwnerReadBinding = { sessionGeneration: connection.session,
    connectionToken: connection.view.connection_token ?? null, owner: connection.view }
  const latestBinding = useRef(binding); latestBinding.current = binding
  const machine = useMemo(() => ({ active: false, visible: !document.hidden, busy: false, paused: true,
    intent: 0, bindingKey: '', timer: null as ReturnType<typeof setTimeout> | null }), [reader])
  const notify = () => { if (machine.active) changed(value => value + 1) }
  const clearTimer = () => { if (machine.timer !== null) clearTimeout(machine.timer); machine.timer = null }
  const key = JSON.stringify([connection.session, connection.view.connection_token, connection.view.paired,
    connection.view.connected, connection.view.hello?.server_instance, connection.view.hello?.workspace_id,
    connection.view.hello?.capabilities.includes('logbook.contact.read')])
  if (machine.bindingKey !== key) {
    machine.bindingKey = key; machine.paused = true; machine.intent++; clearTimer()
  }
  reader.synchronize(binding, selected.current?.id ?? null)
  const allowed = (required: readonly string[]) => {
    const c = current.current.connection, scope = scopeOf(c)
    return scope && c.isCurrentReadConnection(scope, required) ? scope : null
  }
  const pump = useRef<() => void>(() => {})
  pump.current = () => {
    const selection = selected.current
    if (!machine.active || !machine.visible || machine.busy || machine.paused || !selection) return
    if (!allowed(['logbook.contact.read'])) { machine.paused = true; notify(); return }
    clearTimer(); machine.busy = true
    const intent = machine.intent, requestBinding = latestBinding.current
    const same = () => machine.active && intent === machine.intent && selected.current === selection
    void (async () => {
      let success = false
      try {
        const result = await reader.read(requestBinding, selection.id)
        success = same() && !!allowed(['logbook.contact.read'])
          && reader.isCurrent(result, latestBinding.current, selection.id)
      } finally {
        machine.busy = false
        if (!machine.active) return
        notify()
        if (!same()) { if (!machine.paused) queueMicrotask(() => pump.current()); return }
        machine.paused = !success
        if (success && machine.visible) machine.timer = setTimeout(() => pump.current(), 2000)
      }
    })()
    notify()
  }
  useEffect(() => {
    machine.active = true; machine.visible = !document.hidden
    const visibility = () => {
      machine.visible = !document.hidden; clearTimer()
      if (machine.visible) pump.current()
    }
    const timer = setInterval(() => { if (selected.current) notify() }, 100)
    document.addEventListener('visibilitychange', visibility)
    return () => {
      machine.active = false; machine.intent++; clearTimer(); clearInterval(timer)
      document.removeEventListener('visibilitychange', visibility)
      reader.invalidate()
    }
  }, [reader, machine])
  useEffect(() => () => selected.current?.editor.dispose(), [])

  async function mayLeave(): Promise<boolean> {
    const selection = selected.current
    if (!selection) return true
    const policy = selection.editor.closePolicy()
    if (policy === 'blocked') return false
    if (policy !== 'confirm-discard') return true
    const yes = await confirmDialog({ title: t('contactCard.discard.title', { call: selection.editor.inspect().base?.callsign ?? '' }),
      body: t('contactCard.discard.body'), confirmLabel: t('contactCard.discard.confirm'), cancelLabel: t('contactCard.discard.keep') })
    return selected.current === selection && yes && selection.editor.discard()
  }
  async function openExact(id: string, admission: () => boolean): Promise<boolean> {
    if (!canonicalContactId(id) || !admission() || !allowed(['logbook.contact.read'])) return false
    if (!await mayLeave() || !admission() || !allowed(['logbook.contact.read'])) return false
    selected.current?.editor.dispose(); reader.invalidate(); clearTimer()
    const ownerApi = api, ownerReader = reader
    const source = (): OwnerEditorSource | null => {
      if (current.current.api !== ownerApi || current.current.reader !== ownerReader || selected.current?.editor !== editor) return null
      const scope = allowed(['logbook.contact.read'])
      if (!scope) return null
      const b = latestBinding.current, result = reader.inspect(b, id)
      const read = reader.isCurrent(result, b, id) && result.state === 'current'
        ? { detail: result.payload.detail, deadlineMono: result.deadline,
          binding: { token: scope.connectionToken, serverInstance: scope.serverInstance, workspaceId: scope.workspaceId, generation: scope.session } } : null
      return { connection: { token: scope.connectionToken, serverInstance: scope.serverInstance,
        workspaceId: scope.workspaceId, generation: scope.session,
        writeGranted: current.current.connection.isCurrentReadConnection(scope, ['logbook.contact.read', 'logbook.write']) }, read }
    }
    const editor = createOwnerContactEditor({ contactId: id, source, port: api, localTyping: true })
    selected.current = { id, key: `contact-${++serial.current}`, editor }
    machine.intent++; machine.paused = false
    reader.synchronize(latestBinding.current, id); notify(); pump.current()
    return true
  }
  const close = async () => {
    const selection = selected.current
    if (!await mayLeave() || selected.current !== selection) return
    selection?.editor.dispose(); selected.current = null; machine.intent++; machine.paused = true
    clearTimer(); reader.invalidate(); setSaved(loadOwnerEditorDraft()); notify()
  }
  const refresh = () => {
    if (!selected.current || !allowed(['logbook.contact.read'])) return
    machine.intent++; machine.paused = false; reader.invalidate(); clearTimer(); pump.current(); notify()
  }
  const resume = async () => {
    const retained = loadOwnerEditorDraft(); setSaved(retained)
    if (retained.state !== 'current') return false
    const raw = retained.raw, id = retained.value.base.contact_id.toLowerCase()
    return openExact(id, () => {
      const now = loadOwnerEditorDraft(), scope = allowed(['logbook.contact.read'])
      return now.state === 'current' && now.raw === raw && !!scope && now.value.workspace_id === scope.workspaceId
        && !!renderScope && current.current.connection.isCurrentReadConnection(renderScope, ['logbook.contact.read'])
    })
  }
  const selection = selected.current
  const result = reader.inspect(binding, selection?.id ?? null)
  const read: OwnerContactRead = result.state === 'current' && !!allowed(['logbook.contact.read'])
    ? { state: 'current', detail: result.payload.detail }
    : { state: result.state === 'loading' ? 'loading' : 'unavailable', message: t('contactCard.owner.unavailable') }
  const resumeScope = allowed(['logbook.contact.read'])
  const renderScope = scopeOf(connection)
  return { card: selection ? { external: { contactId: selection.id, scopeKey: selection.key, read }, editor: selection.editor } : null,
    close, refresh, refreshEnabled: !!selection && !!resumeScope,
    open: (call: string, id?: string | null) => id ? openExact(id, () => !!renderScope
      && current.current.connection.isCurrentReadConnection(renderScope, ['logbook.contact.read'])
      && current.current.canOpenRow(call, id)) : Promise.resolve(false),
    resume, savedAvailable: !selection && saved.state === 'current',
    resumeEnabled: !selection && saved.state === 'current' && !!resumeScope && saved.value.workspace_id === resumeScope.workspaceId }
}
