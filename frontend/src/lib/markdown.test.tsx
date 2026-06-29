import { render, screen } from '@testing-library/react'
import { describe, it, expect } from 'vitest'
import { MarkdownView } from './markdown'

describe('MarkdownView', () => {
  it('renders allowed elements (heading, emphasis, list, blockquote)', () => {
    render(
      <MarkdownView source={'# Titre\n\n**gras** *ita*\n\n- un\n- deux\n\n> cite'} />,
    )
    expect(screen.getByRole('heading', { name: 'Titre' })).toBeInTheDocument()
    expect(screen.getByText('gras')).toBeInTheDocument()
    expect(screen.getByRole('list')).toBeInTheDocument()
  })

  it('does not render links or images', () => {
    const { container } = render(
      <MarkdownView source={'[x](https://evil.test) ![y](https://evil.test/i.png)'} />,
    )
    expect(container.querySelector('a')).toBeNull()
    expect(container.querySelector('img')).toBeNull()
  })

  it('neutralizes raw HTML / script', () => {
    const { container } = render(
      <MarkdownView source={'<script>window.__x=1</script><b>raw</b>'} />,
    )
    expect(container.querySelector('script')).toBeNull()
    // pas d'exécution : la balise est traitée comme texte, pas comme DOM actif
  })
})
