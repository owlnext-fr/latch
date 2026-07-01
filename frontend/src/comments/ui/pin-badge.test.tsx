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
  it('affiche le label et se positionne', () => {
    render(<PinBadge position={pos} label="A" active={false} onClick={() => {}} />)
    const btn = screen.getByRole('button')
    expect(btn).toHaveTextContent('A')
    expect(btn.style.left).toBe('140px') // 100 + 0.5*80
    expect(btn.style.top).toBe('70px') // 50 + 0.5*40
  })

  it('un pin ancré n’utilise pas la couleur d’avertissement (ambre)', () => {
    render(<PinBadge position={pos} label="A" active={false} onClick={() => {}} />)
    expect(screen.getByRole('button').className).not.toContain('amber')
  })

  it('un pin orphelin passe en ambre', () => {
    render(<PinBadge position={{ ...pos, status: 'orphaned' }} label="J" active={false} onClick={() => {}} />)
    expect(screen.getByRole('button').className).toContain('amber')
  })

  it('appelle onClick', async () => {
    const onClick = vi.fn()
    render(<PinBadge position={pos} label="A" active={false} onClick={onClick} />)
    await userEvent.click(screen.getByRole('button'))
    expect(onClick).toHaveBeenCalledOnce()
  })
})
