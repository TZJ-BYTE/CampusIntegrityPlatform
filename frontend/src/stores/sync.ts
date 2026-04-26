import { defineStore } from 'pinia'
import {
  authGetState,
  authLogin,
  authLogout,
  authSetServer,
  syncGetState,
  syncRun,
  type ApiResponse,
  type AuthState,
  type SyncRunResult,
  type SyncState,
} from '../api/tauri'

export const useSyncStore = defineStore('sync', {
  state: () => ({
    auth: null as ApiResponse<AuthState> | null,
    sync: null as ApiResponse<SyncState> | null,
    lastRun: null as ApiResponse<SyncRunResult> | null,
    busy: false,
    message: '',
    lastOkAt: 0,
    lastErrorAt: 0,
    lastError: '',
    showAdvanced: false,
    serverUrl: '',
    _started: false,
    _timer: 0 as unknown as number,
    _nextDelayMs: 8000,
  }),
  actions: {
    async refresh() {
      this.auth = await authGetState()
      this.sync = await syncGetState()
      if (this.auth.ok) {
        this.serverUrl = this.auth.data.baseUrl ?? ''
      }
    },
    async setServer(baseUrl: string) {
      this.busy = true
      try {
        const r = await authSetServer({ baseUrl })
        this.auth = r
        if (r.ok) {
          this.serverUrl = r.data.baseUrl ?? ''
          this.message = '已保存服务器地址'
        } else {
          this.message = `${r.error.code} · ${r.error.message}`
        }
      } finally {
        this.busy = false
      }
    },
    async login(username?: string, password?: string) {
      this.busy = true
      try {
        const r = await authLogin({ username, password })
        if (r.ok) {
          this.message = '登录成功'
          await this.refresh()
          await this.run('both')
        } else {
          this.message = `${r.error.code} · ${r.error.message}`
        }
      } finally {
        this.busy = false
      }
    },
    async logout() {
      this.busy = true
      try {
        const r = await authLogout()
        if (r.ok) {
          this.message = '已退出登录'
          await this.refresh()
        } else {
          this.message = `${r.error.code} · ${r.error.message}`
        }
      } finally {
        this.busy = false
      }
    },
    async run(mode: 'push' | 'pull' | 'both' = 'both') {
      if (this.busy) return
      if (!this.auth?.ok) return
      if (!this.auth.data.isLoggedIn) return
      this.busy = true
      try {
        this.lastRun = await syncRun({ mode })
        if (this.lastRun.ok) {
          this.lastOkAt = Date.now()
          this.lastErrorAt = 0
          this.lastError = ''
          this.sync = await syncGetState()
          this._nextDelayMs = 8000
        } else {
          this.lastErrorAt = Date.now()
          this.lastError = `${this.lastRun.error.code} · ${this.lastRun.error.message}`
          this._nextDelayMs = Math.min(Math.max(this._nextDelayMs * 2, 8000), 2 * 60 * 1000)
        }
      } finally {
        this.busy = false
      }
    },
    startAuto() {
      if (this._started) return
      this._started = true
      const startedAt = Date.now()
      const bootDelayMs = 6000
      this.refresh()
      let scheduled = 0 as unknown as number
      const schedule = () => {
        window.clearTimeout(scheduled)
        scheduled = window.setTimeout(async () => {
          await this.run('both')
          schedule()
        }, Math.max(this._nextDelayMs, bootDelayMs))
      }
      schedule()
      window.addEventListener('focus', () => {
        if (Date.now() - startedAt < bootDelayMs) return
        this.run('both')
      })
      window.addEventListener('cip-local-write', () => {
        window.clearTimeout((this as any)._debounceTimer)
        ;(this as any)._debounceTimer = window.setTimeout(() => this.run('both'), 800)
      })
      document.addEventListener('visibilitychange', () => {
        if (document.hidden) return
        if (Date.now() - startedAt < bootDelayMs) return
        this.run('both')
      })
    },
  },
})
