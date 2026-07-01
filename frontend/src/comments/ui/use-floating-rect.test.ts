import { describe, expect, it } from 'vitest'
import { floatingMiddleware } from './use-floating-rect'

describe('floatingMiddleware', () => {
  it('compose un pipeline conscient du débordement (borne le viewport)', () => {
    const names = floatingMiddleware().map((m) => m.name)
    expect(names).toEqual(['offset', 'flip', 'shift', 'size'])
  })
})
