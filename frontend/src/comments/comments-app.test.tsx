import { expect, it, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/shell/i18n'
import { CommentsApp } from './comments-app'
import type { CommentsAdapter } from './data/adapter'
import type { FrameRef } from './picker/picker'

const fakeAdapter: CommentsAdapter = {
  capabilities: { canAuthor: true, canEditOwn: true, canModerate: false },
  list: async () => ({ version: 1, pins: [] }),
  createPin: async () => {
    throw new Error('unused')
  },
  addReply: async () => {
    throw new Error('unused')
  },
  editMessage: async () => {
    throw new Error('unused')
  },
  deleteMessage: async () => {},
  deletePin: async () => {},
}

function fakeFrame(): FrameRef {
  const doc = document.implementation.createHTMLDocument('proto')
  doc.body.innerHTML = '<button id="b">Hi</button>'
  return {
    contentDocument: doc,
    contentWindow: { addEventListener() {}, removeEventListener() {} } as unknown as Window,
    getBoundingClientRect: () => ({ left: 0, top: 0, width: 800, height: 600 }) as DOMRect,
  }
}

beforeEach(() => i18n.changeLanguage('en'))

it("monte la barre d'action quand l'adaptateur autorise l'authoring", async () => {
  render(
    <I18nextProvider i18n={i18n}>
      <CommentsApp cacheKey="demo" frame={fakeFrame()} adapter={fakeAdapter} />
    </I18nextProvider>,
  )
  // Le bouton "Comment" n'apparaît que si canAuthor est true
  expect(await screen.findByRole('button', { name: 'Comment' })).toBeInTheDocument()
})
