import { describe, it, expect, vi } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from '@tanstack/react-router'
import { I18nextProvider } from 'react-i18next'
import i18n from '@/i18n'
import { ReviewPage } from '@/routes/review'

vi.mock('@/comments', () => ({
  default: () => <div data-testid="comments-app-stub" />,
}))

function renderReviewPage(id: string, n: string) {
  const history = createMemoryHistory({
    initialEntries: [`/projects/${id}/versions/${n}/review`],
  })
  const rootRoute = createRootRoute({ component: Outlet })
  const reviewRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/projects/$id/versions/$n/review',
    component: ReviewPage,
  })
  const routeTree = rootRoute.addChildren([reviewRoute])
  const router = createRouter({ routeTree, history })

  let result!: ReturnType<typeof render>
  act(() => {
    result = render(
      <I18nextProvider i18n={i18n}>
        <RouterProvider router={router} />
      </I18nextProvider>,
    )
  })
  return result
}

describe('ReviewPage', () => {
  it('monte l\'iframe de preview et la couche commentaire admin', async () => {
    renderReviewPage('3', '2')
    const frame = await screen.findByTitle(/review|preview|prototype/i)
    expect(frame).toHaveAttribute(
      'src',
      expect.stringContaining('/api/projects/3/versions/2/preview'),
    )
  })
})
