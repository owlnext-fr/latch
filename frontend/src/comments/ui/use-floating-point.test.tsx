import { describe, expect, it, vi } from 'vitest'
import { render } from '@testing-library/react'
import { computePosition } from '@floating-ui/dom'
import { floatingMiddleware, PIN_RADIUS, GAP, POPUP_OFFSET, useFloatingPoint } from './use-floating-point'
import { anchorPoint } from './anchor-point'

vi.mock('@floating-ui/dom', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@floating-ui/dom')>()
  return { ...actual, computePosition: vi.fn().mockResolvedValue({ x: 0, y: 0 }) }
})

describe('floatingMiddleware', () => {
  it('compose un pipeline conscient du débordement (borne le viewport)', () => {
    const names = floatingMiddleware().map((m) => m.name)
    expect(names).toEqual(['offset', 'flip', 'shift', 'size'])
  })

  it("borne aussi l'axe horizontal (crossAxis) avec un padding viewport", () => {
    const shift = floatingMiddleware().find((m) => m.name === 'shift')!
    expect(shift.options).toMatchObject({ crossAxis: true, padding: 8 })
  })
})

describe("offset d'ancrage au pin", () => {
  it('dégage le rayon du pin plus un gap', () => {
    expect(PIN_RADIUS).toBe(14)
    expect(GAP).toBe(8)
    expect(POPUP_OFFSET).toBe(PIN_RADIUS + GAP)
    expect(POPUP_OFFSET).toBe(22)
  })
})

function Harness({
  rect,
  offset,
}: Readonly<{
  rect: { x: number; y: number; width: number; height: number }
  offset: { x: number; y: number }
}>) {
  // Reproduit ThreadPopup : point recalculé inline (nouvel objet) à CHAQUE rendu.
  const { ref } = useFloatingPoint(anchorPoint(rect, offset))
  return <div ref={ref} />
}

describe('useFloatingPoint — anti-boucle', () => {
  it('ne recalcule pas la position en boucle quand le point est recréé à chaque rendu (anti-boucle)', async () => {
    const rect = { x: 100, y: 50, width: 400, height: 300 }
    const offset = { x: 0.5, y: 0.5 }
    render(<Harness rect={rect} offset={offset} />)
    // laisse les microtâches (setStyle → re-render) se stabiliser
    await new Promise((resolve) => setTimeout(resolve, 30))
    expect(computePosition).toHaveBeenCalledTimes(1)
  })
})
