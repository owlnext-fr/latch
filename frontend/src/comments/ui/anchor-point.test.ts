import { describe, expect, it } from 'vitest'
import { anchorPoint } from './anchor-point'

describe('anchorPoint', () => {
  const rect = { x: 100, y: 50, width: 80, height: 40 }

  it('coin haut-gauche pour offset {0,0}', () => {
    expect(anchorPoint(rect, { x: 0, y: 0 })).toEqual({ x: 100, y: 50 })
  })

  it('centre pour offset {0.5,0.5}', () => {
    expect(anchorPoint(rect, { x: 0.5, y: 0.5 })).toEqual({ x: 140, y: 70 })
  })

  it('coin bas-droit pour offset {1,1}', () => {
    expect(anchorPoint(rect, { x: 1, y: 1 })).toEqual({ x: 180, y: 90 })
  })
})
