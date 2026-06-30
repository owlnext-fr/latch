import { useEffect, useRef, useState } from 'react'
import { FollowController, type PinInput, type PinPosition } from './controller'
import type { Picker } from '../picker/picker'

export function useFollow(picker: Picker | null, pins: PinInput[]): PinPosition[] {
  const [positions, setPositions] = useState<PinPosition[]>([])
  const ctrlRef = useRef<FollowController | null>(null)
  // Always holds the latest pins so Effect 1 can prime the controller before start().
  const pinsRef = useRef<PinInput[]>(pins)
  pinsRef.current = pins

  useEffect(() => {
    if (!picker) {
      setPositions([])
      return
    }
    const ctrl = new FollowController(picker)
    ctrlRef.current = ctrl
    const off = ctrl.onUpdate((next) =>
      setPositions((prev) =>
        JSON.stringify(prev) === JSON.stringify(next) ? prev : next,
      ),
    )
    // Prime pins before start() so the first measurement is correct.
    ctrl.setPins(pinsRef.current)
    ctrl.start()
    return () => {
      off()
      ctrl.stop()
      ctrlRef.current = null
    }
  }, [picker])

  useEffect(() => {
    ctrlRef.current?.setPins(pins)
  }, [pins])

  return positions
}
