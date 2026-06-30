import { describe, expect, it, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { OverlayLayer } from './overlay-layer'
import type { Picker, ShellRect } from '../picker/picker'
import type { PinPosition } from '../follow/controller'

const anchor = {
  v: 1 as const,
  selector: '#x',
  fingerprint: { tag: 'div', text: '', role: null, ordinal: 0 },
  textQuote: null,
  offset: { x: 0.5, y: 0.5 },
  fallbackPoint: { x: 0, y: 0 },
}

function fakePicker(over: Partial<Picker> = {}): Picker {
  const el = {} as Element
  return {
    doc: document,
    getElementAt: () => el,
    describe: () => anchor,
    resolve: () => ({ element: el, status: 'anchored' }),
    toShellRect: (): ShellRect => ({ x: 1, y: 2, width: 3, height: 4 }),
    fallbackRect: (): ShellRect => ({ x: 0, y: 0, width: 0, height: 0 }),
    subscribe: () => () => {},
    ...over,
  }
}

const positions: PinPosition[] = [
  { id: 5, status: 'anchored', rect: { x: 10, y: 10, width: 20, height: 20 }, offset: { x: 0.5, y: 0.5 } },
]

describe('OverlayLayer', () => {
  it('renders a badge per position', () => {
    render(
      <OverlayLayer
        picker={fakePicker()}
        positions={positions}
        pickMode={false}
        onPick={vi.fn()}
        onPinClick={vi.fn()}
        activePinId={null}
      />,
    )
    expect(screen.getByRole('button')).toBeInTheDocument()
  })

  it('captures an anchor on click in pick mode', () => {
    const onPick = vi.fn()
    const { container } = render(
      <OverlayLayer
        picker={fakePicker()}
        positions={[]}
        pickMode
        onPick={onPick}
        onPinClick={vi.fn()}
        activePinId={null}
      />,
    )
    const surface = container.querySelector('[data-testid="pick-surface"]')!
    fireEvent.click(surface, { clientX: 50, clientY: 60 })
    expect(onPick).toHaveBeenCalledWith(anchor, { x: 1, y: 2, width: 3, height: 4 })
  })

  it('forwards pin clicks', async () => {
    const onPinClick = vi.fn()
    render(
      <OverlayLayer
        picker={fakePicker()}
        positions={positions}
        pickMode={false}
        onPick={vi.fn()}
        onPinClick={onPinClick}
        activePinId={null}
      />,
    )
    fireEvent.click(screen.getByRole('button'))
    expect(onPinClick).toHaveBeenCalledWith(5)
  })
})
