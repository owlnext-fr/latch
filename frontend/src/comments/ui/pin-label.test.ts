import { describe, expect, it } from 'vitest'
import { firstLetter } from './pin-label'

describe('firstLetter', () => {
  it('renvoie la première lettre en majuscule', () => {
    expect(firstLetter('alice')).toBe('A')
    expect(firstLetter('  léa ')).toBe('L')
  })
  it('retombe sur • pour un nom vide', () => {
    expect(firstLetter('')).toBe('•')
    expect(firstLetter('   ')).toBe('•')
  })
})
