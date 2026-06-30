import { describe, expect, it, beforeEach } from 'vitest'
import { http, HttpResponse } from 'msw'
import { render, screen } from '@testing-library/react'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/shell/i18n'
import { server } from '@/test/msw'
import { CommentsApp } from './comments-app'
import type { FrameRef } from './picker/picker'

const ORIGIN = globalThis.location.origin
const SLUG = 'demo-aB3dEf9z'

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

describe('CommentsApp', () => {
  it('renders the action bar with the loaded pin count', async () => {
    server.use(
      http.get(`${ORIGIN}/c/${SLUG}/comments`, () =>
        HttpResponse.json(
          {
            version: 1,
            pins: [
              {
                id: 1,
                anchor: JSON.stringify({
                  v: 1,
                  selector: '#b',
                  fingerprint: { tag: 'button', text: 'Hi', role: 'button', ordinal: 0 },
                  textQuote: null,
                  offset: { x: 0.5, y: 0.5 },
                  fallbackPoint: { x: 0, y: 0 },
                }),
                created_at: 'n',
                messages: [
                  { id: 9, author_name: 'Léa', body: 'Hi', created_at: 'n', updated_at: 'n', editable: true },
                ],
              },
            ],
          },
          { status: 200 },
        ),
      ),
    )
    render(
      <I18nextProvider i18n={i18n}>
        <CommentsApp slug={SLUG} frame={fakeFrame()} />
      </I18nextProvider>,
    )
    expect(await screen.findByText('1 comment')).toBeInTheDocument()
  })
})
