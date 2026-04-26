import http from 'node:http'
import fs from 'node:fs'
import path from 'node:path'
import { URL } from 'node:url'
import crypto from 'node:crypto'

const port = Number.parseInt(process.env.PORT || '8788', 10)
const repoDir = process.env.SYNC_REPO_DIR
  ? path.resolve(process.env.SYNC_REPO_DIR)
  : path.join(process.cwd(), 'server', 'sync-repo')
const eventsPath = path.join(repoDir, 'events.json')

fs.mkdirSync(repoDir, { recursive: true })

function readJsonFile(file, fallback) {
  try {
    const text = fs.readFileSync(file, 'utf-8')
    return JSON.parse(text)
  } catch {
    return fallback
  }
}

function writeJsonFileAtomic(file, data) {
  const tmp = `${file}.tmp`
  fs.writeFileSync(tmp, JSON.stringify(data, null, 2), 'utf-8')
  fs.renameSync(tmp, file)
}

const store = readJsonFile(eventsPath, { nextSeq: 1, events: [] })
const eventIds = new Set(store.events.map((e) => e.eventId))
const tokens = new Map()

function json(res, status, obj) {
  const body = JSON.stringify(obj)
  res.statusCode = status
  res.setHeader('Content-Type', 'application/json; charset=utf-8')
  res.setHeader('Content-Length', Buffer.byteLength(body))
  res.end(body)
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let data = ''
    req.on('data', (chunk) => {
      data += chunk
      if (data.length > 2 * 1024 * 1024) reject(new Error('body too large'))
    })
    req.on('end', () => resolve(data))
    req.on('error', reject)
  })
}

function getBearerToken(req) {
  const h = req.headers['authorization']
  if (!h) return null
  const s = Array.isArray(h) ? h.join(',') : h
  const m = /^Bearer\s+(.+)$/.exec(s)
  return m ? m[1] : null
}

function requireAuth(req, res) {
  const t = getBearerToken(req)
  if (!t || !tokens.has(t)) {
    json(res, 401, { error: 'unauthorized' })
    return null
  }
  return t
}

const server = http.createServer(async (req, res) => {
  try {
    const u = new URL(req.url || '/', `http://${req.headers.host || '127.0.0.1'}`)
    const p = u.pathname
    if (req.method === 'POST' && p === '/v1/auth/login') {
      const text = await readBody(req)
      const body = text ? JSON.parse(text) : {}
      const username = String(body.username || 'user')
      const token = crypto.randomUUID()
      tokens.set(token, { username, createdAt: Date.now() })
      return json(res, 200, { accessToken: token })
    }

    if (req.method === 'POST' && p === '/v1/sync/push') {
      if (!requireAuth(req, res)) return
      const text = await readBody(req)
      const body = text ? JSON.parse(text) : {}
      const events = Array.isArray(body.events) ? body.events : []
      const acked = []
      for (const e of events) {
        const eventId = String(e.eventId || e.event_id || '')
        if (!eventId) continue
        acked.push(eventId)
        if (eventIds.has(eventId)) continue
        eventIds.add(eventId)
        const stored = {
          seq: store.nextSeq++,
          eventId,
          deviceId: String(e.deviceId || e.device_id || ''),
          eventType: String(e.eventType || e.event_type || ''),
          entityType: String(e.entityType || e.entity_type || ''),
          entityId: String(e.entityId || e.entity_id || ''),
          payload: e.payload ?? {},
          occurredAt: Number(e.occurredAt ?? e.occurred_at ?? Date.now()),
        }
        store.events.push(stored)
      }
      writeJsonFileAtomic(eventsPath, store)
      return json(res, 200, { acked })
    }

    if (req.method === 'GET' && p === '/v1/sync/pull') {
      if (!requireAuth(req, res)) return
      const cursor = Number(u.searchParams.get('cursor') || '0')
      const page = store.events
        .filter((e) => e.seq > cursor)
        .sort((a, b) => (a.seq - b.seq) || String(a.eventId).localeCompare(String(b.eventId)))
        .slice(0, 500)
      const nextCursor = page.length ? String(page[page.length - 1].seq) : String(cursor)
      const events = page.map((e) => ({
        eventId: e.eventId,
        deviceId: e.deviceId,
        eventType: e.eventType,
        entityType: e.entityType,
        entityId: e.entityId,
        payload: e.payload ?? {},
        occurredAt: e.occurredAt,
      }))
      return json(res, 200, { events, nextCursor })
    }

    json(res, 404, { error: 'not found' })
  } catch (e) {
    json(res, 500, { error: String(e?.message || e) })
  }
})

server.listen(port, '127.0.0.1', () => {
  process.stdout.write(`sync server listening on http://127.0.0.1:${port}\n`)
  process.stdout.write(`repoDir: ${repoDir}\n`)
})

