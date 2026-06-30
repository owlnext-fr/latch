import type { AnchorDescriptor } from '../anchor/descriptor'
import type { ShellRect } from '../picker/picker'

export type PickState =
  | { mode: 'idle' }
  | { mode: 'pick' }
  | { mode: 'compose'; anchor: AnchorDescriptor; rect: ShellRect }

export type PickEvent =
  | { type: 'ENTER_PICK' }
  | { type: 'CANCEL' }
  | { type: 'CAPTURE'; anchor: AnchorDescriptor; rect: ShellRect }
  | { type: 'SUBMITTED' }

export const initialPickState: PickState = { mode: 'idle' }

export function pickReducer(state: PickState, event: PickEvent): PickState {
  switch (event.type) {
    case 'ENTER_PICK':
      return { mode: 'pick' }
    case 'CANCEL':
    case 'SUBMITTED':
      return { mode: 'idle' }
    case 'CAPTURE':
      if (state.mode !== 'pick') return state
      return { mode: 'compose', anchor: event.anchor, rect: event.rect }
    default:
      return state
  }
}
