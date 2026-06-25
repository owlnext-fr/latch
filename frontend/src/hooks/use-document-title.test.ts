import { describe, it, expect } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useDocumentTitle } from './use-document-title'

describe('useDocumentTitle', () => {
  it('sets document.title to the given value', () => {
    renderHook(() => useDocumentTitle('Hello — latch admin'))
    expect(document.title).toBe('Hello — latch admin')
  })

  it('updates the title when the value changes', () => {
    const { rerender } = renderHook(({ t }) => useDocumentTitle(t), {
      initialProps: { t: 'First' },
    })
    expect(document.title).toBe('First')
    rerender({ t: 'Second' })
    expect(document.title).toBe('Second')
  })
})
