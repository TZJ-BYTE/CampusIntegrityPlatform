import http from 'node:http'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const repoDir = process.env.CONTENT_REPO_DIR
  ? path.resolve(process.env.CONTENT_REPO_DIR)
  : path.resolve(__dirname, 'content-repo')

const port = Number(process.env.PORT || 8787)

function send(res, status, headers, body) {
  res.writeHead(status, headers)
  res.end(body)
}

function sendJson(res, status, obj) {
  send(res, status, { 'content-type': 'application/json; charset=utf-8' }, JSON.stringify(obj, null, 2))
}

function notFound(res) {
  sendJson(res, 404, { ok: false, error: { code: 'NOT_FOUND', message: 'not found' } })
}

function safeJoin(base, p) {
  const target = path.resolve(base, '.' + p)
  if (!target.startsWith(base)) return null
  return target
}

const server = http.createServer((req, res) => {
  try {
    const u = new URL(req.url || '/', `http://${req.headers.host || '127.0.0.1'}`)
    if (req.method !== 'GET') return notFound(res)

    if (u.pathname === '/' || u.pathname === '/health') {
      return sendJson(res, 200, { ok: true, repoDir })
    }

    if (u.pathname === '/versions.json') {
      const p = safeJoin(repoDir, '/versions.json')
      if (!p) return notFound(res)
      if (!fs.existsSync(p)) return notFound(res)
      const text = fs.readFileSync(p, 'utf-8')
      return send(res, 200, { 'content-type': 'application/json; charset=utf-8' }, text)
    }

    if (u.pathname === '/content-pack.zip') {
      const p = safeJoin(repoDir, '/content-pack.zip')
      if (!p) return notFound(res)
      if (!fs.existsSync(p)) return notFound(res)
      const buf = fs.readFileSync(p)
      return send(res, 200, { 'content-type': 'application/zip' }, buf)
    }

    notFound(res)
  } catch (e) {
    sendJson(res, 500, { ok: false, error: { code: 'INTERNAL', message: String(e) } })
  }
})

server.listen(port, '127.0.0.1', () => {
  process.stdout.write(`content-update-server listening on http://127.0.0.1:${port}\n`)
  process.stdout.write(`repoDir: ${repoDir}\n`)
})

