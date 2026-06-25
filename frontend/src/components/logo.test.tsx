import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Logo } from './logo'

describe('Logo', () => {
  it('renders an image with the latch alt text', () => {
    render(<Logo className="size-6" />)
    const img = screen.getByRole('img', { name: 'latch' })
    expect(img).toBeInTheDocument()
    expect(img).toHaveClass('size-6')
  })
})
