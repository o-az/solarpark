/**
 * deploying with `deno deploy`
 *
 * preview deployment: `deno deploy --app solar`
 * production deployment: `deno deploy --app solar --prod`
 *
 * https://solar.zk.deno.net
 */

import { serveDir } from '@std/http/file-server'

Deno.serve((request) => {
  return serveDir(request, {
    fsRoot: import.meta.dirname,
  })
})
