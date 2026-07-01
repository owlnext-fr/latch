import { describe, expect, it } from 'vitest'
import { floatingMiddleware, PIN_RADIUS, GAP, POPUP_OFFSET } from './use-floating-point'

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
