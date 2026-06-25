import { createRootRoute, createRoute, createRouter, Outlet } from '@tanstack/react-router'
import { LoginPage } from './routes/login'
import { ListPage } from './routes/list'
import { DetailPage } from './routes/detail'

const rootRoute = createRootRoute({ component: Outlet })
const loginRoute = createRoute({ getParentRoute: () => rootRoute, path: '/login', component: LoginPage })
const listRoute = createRoute({ getParentRoute: () => rootRoute, path: '/', component: ListPage })
const detailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/projects/$id',
  component: DetailPage,
})

const routeTree = rootRoute.addChildren([loginRoute, listRoute, detailRoute])

export const router = createRouter({ routeTree, basepath: '/admin' })

declare module '@tanstack/react-router' {
  interface Register { router: typeof router }
}
