import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PinBadge } from './pin-badge'
import type { PinPosition } from '../follow/controller'

const pos: PinPosition = {
  id: 1,
  status: 'anchored',
  rect: { x: 100, y: 50, width: 80, height: 40 },
  offset: { x: 0.5, y: 0.5 },
}

describe('PinBadge', () => {
  it('renders the message count and positions itself', () => {
    render(<PinBadge position={pos} count={3} active={false} onClick={() => {}} />)
    const btn = screen.getByRole('button')
    expect(btn).toHaveTextContent('3')
    // 100 + 0.5*80 = 140 ; 50 + 0.5*40 = 70
    expect(btn.style.left).toBe('140px')
    expect(btn.style.top).toBe('70px')
  })

  it('calls onClick', async () => {
    const onClick = vi.fn()
    render(<PinBadge position={pos} count={1} active={false} onClick={onClick} />)
    await userEvent.click(screen.getByRole('button'))
    expect(onClick).toHaveBeenCalledOnce()
  })

  it('marks a moved (approximate) pin via data-status', () => {
    render(
      <PinBadge position={{ ...pos, status: 'approximate' }} count={1} active={false} onClick={() => {}} />,
    )
    expect(screen.getByRole('button')).toHaveAttribute('data-status', 'approximate')
  })
})
