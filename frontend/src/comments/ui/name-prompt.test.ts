import { describe, expect, it, beforeEach } from 'vitest'
import { getStoredName, setStoredName } from './name-prompt'

beforeEach(() => localStorage.clear())

describe('name prompt storage', () => {
  it('returns empty string when nothing stored', () => {
    expect(getStoredName()).toBe('')
  })

  it('persists and reads back a name', () => {
    setStoredName('Léa')
    expect(getStoredName()).toBe('Léa')
  })

  it('trims on write', () => {
    setStoredName('  Léa  ')
    expect(getStoredName()).toBe('Léa')
  })
})
