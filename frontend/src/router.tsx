import {
  createRootRoute,
  createRoute,
  createRouter,
  lazyRouteComponent,
  Outlet,
} from '@tanstack/react-router'

// Routes code-splittées : chaque page est chargée à la demande (chunk séparé)
// pour alléger le First Load JS. Exports nommés → 2e argument de lazyRouteComponent.
const rootRoute = createRootRoute({ component: Outlet })
const loginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/login',
  component: lazyRouteComponent(() => import('./routes/login'), 'LoginPage'),
})
const listRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: lazyRouteComponent(() => import('./routes/list'), 'ListPage'),
})
const detailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/projects/$id',
  component: lazyRouteComponent(() => import('./routes/detail'), 'DetailPage'),
})
const reviewRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/projects/$id/versions/$n/review',
  component: lazyRouteComponent(() => import('./routes/review'), 'ReviewPage'),
})

const routeTree = rootRoute.addChildren([loginRoute, listRoute, detailRoute, reviewRoute])

// Filet anti-flash pendant le chargement d'un chunk de route (réseau lent).
// Volontairement trivial : aucun import lourd ici, sinon le split est défait.
function RoutePending() {
  return <div className="min-h-svh bg-background" aria-busy="true" />
}

export const router = createRouter({
  routeTree,
  basepath: '/admin',
  defaultPendingComponent: RoutePending,
})

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
