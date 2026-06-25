import { describe, expect, it } from 'vitest'
import { humanSize, publicUrl } from './utils'

describe('humanSize', () => {
  it('formats bytes/KB/MB', () => {
    expect(humanSize(512)).toBe('512 B')
    expect(humanSize(2048)).toBe('2.0 KB')
    expect(humanSize(5 * 1024 * 1024)).toBe('5.0 MB')
  })
})

describe('publicUrl', () => {
  it('builds /c/<slug> on current origin', () => {
    expect(publicUrl('mon-projet-k7Qp2maZ')).toContain('/c/mon-projet-k7Qp2maZ')
  })
})
