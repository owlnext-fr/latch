import { describe, expect, it, beforeEach } from 'vitest'
import { http, HttpResponse } from 'msw'
import { render, screen } from '@testing-library/react'
import { I18nextProvider } from 'react-i18next'
import i18n from './i18n'
import { server } from '@/test/msw'
import { CommentsMount } from './comments-mount'

const ORIGIN = globalThis.location.origin
const SLUG = 'demo-aB3dEf9z'

beforeEach(() => {
  i18n.changeLanguage('en')
  server.use(
    http.get(`${ORIGIN}/c/${SLUG}/comments`, () =>
      HttpResponse.json({ version: 1, pins: [] }, { status: 200 }),
    ),
  )
})

describe('CommentsMount', () => {
  it('lazy-loads the comments module and renders its action bar', async () => {
    const iframe = document.createElement('iframe')
    document.body.appendChild(iframe)
    render(
      <I18nextProvider i18n={i18n}>
        <CommentsMount slug={SLUG} frame={iframe} />
      </I18nextProvider>,
    )
    // le chunk se charge async (Suspense) ; la barre apparaît une fois la liste chargée
    expect(await screen.findByText('0 comments')).toBeInTheDocument()
  })
})
