/**
 * deploying with `deno deploy`
 *
 * preview deployment: `deno deploy --app solar`
 * production deployment: `deno deploy --app solar --prod`
 *
 * https://solar.zk.deno.net
 */

import { serveDir } from "@std/http/file-server";

const MIME_TYPES: Record<string, string> = {
  ".js": "application/javascript",
  ".mjs": "application/javascript",
  ".wasm": "application/wasm",
  ".json": "application/json",
  ".html": "text/html",
  ".css": "text/css",
};

Deno.serve(async (request) => {
  const response = await serveDir(request, {
    fsRoot: import.meta.dirname,
  });

  const url = new URL(request.url);
  const ext = url.pathname.slice(url.pathname.lastIndexOf("."));
  const mimeType = MIME_TYPES[ext];

  if (
    mimeType && response.headers.get("content-type")?.startsWith("text/plain")
  ) {
    const newHeaders = new Headers(response.headers);
    newHeaders.set("content-type", mimeType);
    return new Response(response.body, {
      status: response.status,
      statusText: response.statusText,
      headers: newHeaders,
    });
  }

  return response;
});
