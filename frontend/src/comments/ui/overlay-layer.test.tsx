import { describe, expect, it, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { OverlayLayer, glowShadow } from './overlay-layer'
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
  {
    id: 5,
    status: 'anchored',
    rect: { x: 10, y: 10, width: 20, height: 20 },
    offset: { x: 0.5, y: 0.5 },
  },
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
        labelOf={() => 'A'}
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
        labelOf={() => 'A'}
      />,
    )
    const surface = container.querySelector('[data-testid="pick-surface"]')!
    fireEvent.click(surface, { clientX: 50, clientY: 60 })
    expect(onPick).toHaveBeenCalledWith(anchor, {
      x: 1,
      y: 2,
      width: 3,
      height: 4,
    })
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
        labelOf={() => 'A'}
      />,
    )
    fireEvent.click(screen.getByRole('button'))
    expect(onPinClick).toHaveBeenCalledWith(5)
  })

  it("ancre l'overlay au viewport (fixed) pour ignorer l'offset du conteneur (topbar admin)", () => {
    const { container } = render(
      <OverlayLayer
        picker={fakePicker()}
        positions={[]}
        pickMode={false}
        onPick={vi.fn()}
        onPinClick={vi.fn()}
        activePinId={null}
        labelOf={() => 'A'}
      />,
    )
    const root = container.firstElementChild as HTMLElement
    expect(root.className).toContain('fixed')
    expect(root.className).not.toContain('absolute')
  })

  it('ne rend pas les pins hors écran (hidden) — évite le pin collé en (0,0)', () => {
    render(
      <OverlayLayer
        picker={fakePicker()}
        positions={[
          { id: 9, status: 'anchored', rect: { x: 0, y: 0, width: 0, height: 0 }, offset: { x: 0.5, y: 0.5 }, hidden: true },
        ]}
        pickMode={false}
        onPick={vi.fn()}
        onPinClick={vi.fn()}
        activePinId={null}
        labelOf={() => 'A'}
      />,
    )
    expect(screen.queryByTestId('pin-badge')).toBeNull()
  })
})

describe('ciblage DOM (glow)', () => {
  it('cape la profondeur du glow (non proportionnelle)', () => {
    expect(glowShadow(20, 20)).toContain('6px') // 0.3*20 = 6
    expect(glowShadow(1000, 1000)).toContain('30px') // capé à 30
    expect(glowShadow(20, 20)).toContain('inset')
  })

  it('rend un highlight fluo au survol en pick mode', () => {
    const { container } = render(
      <OverlayLayer
        picker={fakePicker()}
        positions={[]}
        pickMode
        onPick={vi.fn()}
        onPinClick={vi.fn()}
        activePinId={null}
        labelOf={() => 'A'}
      />,
    )
    const surface = container.querySelector('[data-testid="pick-surface"]')!
    fireEvent.mouseMove(surface, { clientX: 5, clientY: 6 })
    const hl = container.querySelector(
      '[data-testid="pick-highlight"]',
    ) as HTMLElement
    expect(hl).not.toBeNull()
    expect(hl.style.boxShadow).toContain('inset')
    // jsdom normalise systématiquement les couleurs CSS en rgb(), y compris dans
    // `border` — #18A0FB == rgb(24, 160, 251). On vérifie donc cette forme normalisée
    // plutôt que la casse hexadécimale d'origine (cf. task-2-brief.md note jsdom).
    expect(hl.style.border).toContain('rgb(24, 160, 251)')
  })
})
