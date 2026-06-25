import { setupServer } from 'msw/node'
import { http, HttpResponse, type JsonBodyType } from 'msw'

export const server = setupServer()

/** Register a one-shot JSON handler for the next request to `url`. */
export function jsonOnce(url: string, body: JsonBodyType, status = 200) {
  server.use(
    http.get(url, () => HttpResponse.json(body, { status }), { once: true }),
  )
}
