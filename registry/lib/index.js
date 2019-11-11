/* global addEventListener */

import * as dnsQuery from './routes/dns-query'
import * as update from './routes/update'
import { respond } from './util'

const handle = async req => {
  const url = req.parsedURL = new URL(req.url)

  switch ((req.method, url.pathname)) {
    case ('GET', '/v1/dns-query'):
      return dnsQuery.handle(req)
    case ('POST', '/v1/update'):
      return update.handle(req)
    default:
      return respond(400)
  }
}

addEventListener('fetch', event => {
  event.respondWith(handle(event.request))
})
