import { createRootRoute, createRoute, createRouter, Outlet } from '@tanstack/react-router'
import { LoginPage } from './routes/login'
import { ListPage } from './routes/list'
import { DetailPage } from './routes/detail'
import { ReviewPage } from './routes/review'

const rootRoute = createRootRoute({ component: Outlet })
const loginRoute = createRoute({ getParentRoute: () => rootRoute, path: '/login', component: LoginPage })
const listRoute = createRoute({ getParentRoute: () => rootRoute, path: '/', component: ListPage })
const detailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/projects/$id',
  component: DetailPage,
})
const reviewRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/projects/$id/versions/$n/review',
  component: ReviewPage,
})

const routeTree = rootRoute.addChildren([loginRoute, listRoute, detailRoute, reviewRoute])

export const router = createRouter({ routeTree, basepath: '/admin' })

declare module '@tanstack/react-router' {
  interface Register { router: typeof router }
}
