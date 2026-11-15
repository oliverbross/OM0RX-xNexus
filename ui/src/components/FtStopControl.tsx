/** The FT stop: Stop TX, which cuts even an over already in flight. */
import { OPERATE_UNAVAILABLE, useOperateServices } from './operateServices'
export function FtStopControl({ onHaltTx }: { onHaltTx?: () => void }) {
  const { canAction } = useOperateServices()
  return (
    <button
      type="button"
      className="op-btn stop"
      data-stop-control="stop-tx"
      disabled={!canAction('stopTransmit')}
      onClick={() => onHaltTx?.()}
      title={canAction('stopTransmit') ? 'Stop transmitting immediately — cuts even an over already in flight' : OPERATE_UNAVAILABLE}
    >Stop TX</button>
  )
}
