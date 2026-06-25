import { createRootRoute, createRoute, createRouter, Outlet } from '@tanstack/react-router'
import { LoginPage } from './routes/login'
import { ListPage } from './routes/list'
import { DetailPage } from './routes/detail'
import { SettingsPage } from './routes/settings'

const rootRoute = createRootRoute({ component: Outlet })
const loginRoute = createRoute({ getParentRoute: () => rootRoute, path: '/login', component: LoginPage })
const listRoute = createRoute({ getParentRoute: () => rootRoute, path: '/', component: ListPage })
const detailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/projects/$id',
  component: DetailPage,
})
const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/settings',
  component: SettingsPage,
})

const routeTree = rootRoute.addChildren([loginRoute, listRoute, detailRoute, settingsRoute])

export const router = createRouter({ routeTree, basepath: '/admin' })

declare module '@tanstack/react-router' {
  interface Register { router: typeof router }
}
