import { useEffect, useRef, useState } from 'react'
import { FollowController, type PinInput, type PinPosition } from './controller'
import type { Picker } from '../picker/picker'

export function useFollow(picker: Picker | null, pins: PinInput[]): PinPosition[] {
  const [positions, setPositions] = useState<PinPosition[]>([])
  const ctrlRef = useRef<FollowController | null>(null)

  // Effect 1: create/destroy the controller whenever picker or pins changes.
  // pins is included in deps so the closure always has the latest value and the
  // controller is primed before start() — this prevents a setState flip from [] to
  // real positions that would cause an infinite render loop when the caller passes
  // unstable picker/pins references (e.g. in tests).
  useEffect(() => {
    if (!picker) return
    const ctrl = new FollowController(picker)
    ctrlRef.current = ctrl
    const off = ctrl.onUpdate((next) =>
      setPositions((prev) =>
        JSON.stringify(prev) === JSON.stringify(next) ? prev : next,
      ),
    )
    ctrl.setPins(pins)
    ctrl.start()
    return () => {
      off()
      ctrl.stop()
      ctrlRef.current = null
    }
  }, [picker, pins])

  return picker ? positions : []
}
