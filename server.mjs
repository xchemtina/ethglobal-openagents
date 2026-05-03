// Minimal static file server that treats /world-map-demo as the project root.
// Serves world-map.html at "/" and exposes every relative asset
// (world-model.json, dft/*, molecules/*, etc.) the page fetches.
import http from "node:http";
import { promises as fs } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.join(__dirname, "world-map-demo");
const PORT = Number(process.env.PORT) || 3000;
const HOST = process.env.HOST || "0.0.0.0";
const DEFAULT_FILE = "world-map.html";

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".htm": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".map": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".webp": "image/webp",
  ".ico": "image/x-icon",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".ttf": "font/ttf",
  ".otf": "font/otf",
  ".txt": "text/plain; charset=utf-8",
  ".md": "text/markdown; charset=utf-8",
  ".xyz": "text/plain; charset=utf-8",
  ".cube": "text/plain; charset=utf-8",
  ".gz": "application/gzip",
  ".wasm": "application/wasm",
}

function safeJoin(root, requestPath) {
  // Strip query string, decode, and prevent directory traversal.
  const clean = decodeURIComponent(requestPath.split("?")[0].split("#")[0])
  const joined = path.join(root, clean)
  const resolved = path.resolve(joined)
  if (!resolved.startsWith(path.resolve(root))) return null
  return resolved
}

async function send(res, status, body, headers = {}) {
  res.writeHead(status, { "Cache-Control": "no-store", ...headers })
  res.end(body)
}

const server = http.createServer(async (req, res) => {
  try {
    let urlPath = req.url || "/"
    if (urlPath === "/" || urlPath === "") urlPath = `/${DEFAULT_FILE}`

    let filePath = safeJoin(ROOT, urlPath)
    if (!filePath) return send(res, 403, "Forbidden")

    let stat
    try {
      stat = await fs.stat(filePath)
    } catch {
      return send(res, 404, `Not found: ${urlPath}`)
    }

    if (stat.isDirectory()) {
      filePath = path.join(filePath, "index.html")
      try {
        stat = await fs.stat(filePath)
      } catch {
        return send(res, 404, `Not found: ${urlPath}`)
      }
    }

    const ext = path.extname(filePath).toLowerCase()
    const type = MIME[ext] || "application/octet-stream"
    const data = await fs.readFile(filePath)
    return send(res, 200, data, { "Content-Type": type })
  } catch (err) {
    console.error("[server] error", err)
    return send(res, 500, "Internal Server Error")
  }
})

server.listen(PORT, HOST, () => {
  console.log(`[server] serving ${ROOT} at http://${HOST}:${PORT}`)
  console.log(`[server] default file: /${DEFAULT_FILE}`)
})
