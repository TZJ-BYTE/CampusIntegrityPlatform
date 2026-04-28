import { defineStore } from 'pinia'
import { contentApplyPack, contentCheckUpdate, contentDownloadUpdate } from '../api/tauri'
import { useAppStore } from './app'
import { getServerBaseUrl } from '../utils/serverBaseUrl'

export const useContentUpdateStore = defineStore('contentUpdate', {
  state: () => ({
    busy: false,
    lastAttemptAt: 0,
    lastSuccessAt: 0,
    _started: false,
    _timer: 0 as unknown as number,
  }),
  actions: {
    async tick() {
      if (this.busy) return
      const now = Date.now()
      if (this.lastAttemptAt && now - this.lastAttemptAt < 10 * 60 * 1000) return
      if (this.lastSuccessAt && now - this.lastSuccessAt < 6 * 60 * 60 * 1000) return
      this.busy = true
      this.lastAttemptAt = now
      try {
        const info = await contentCheckUpdate({ baseUrl: getServerBaseUrl() })
        if (!info.ok) return
        this.lastSuccessAt = now
        if (!info.data.hasUpdate) return
        const d = await contentDownloadUpdate({ url: info.data.downloadUrl })
        if (!d.ok) return
        const a = await contentApplyPack({ packPath: d.data.packPath })
        if (!a.ok) return
        await useAppStore().refreshStatus()
      } finally {
        this.busy = false
      }
    },
    startAuto() {
      if (this._started) return
      this._started = true
      const startedAt = Date.now()
      const bootDelayMs = 30 * 1000
      const tick = () => this.tick()
      window.setTimeout(tick, bootDelayMs)
      this._timer = window.setInterval(tick, 10 * 60 * 1000)
      window.addEventListener('focus', () => {
        if (Date.now() - startedAt < bootDelayMs) return
        tick()
      })
      document.addEventListener('visibilitychange', () => {
        if (document.hidden) return
        if (Date.now() - startedAt < bootDelayMs) return
        tick()
      })
    },
  },
})
