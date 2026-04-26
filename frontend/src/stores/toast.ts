import { defineStore } from 'pinia'

export type ToastKind = 'info' | 'success' | 'error'

export type ToastItem = {
  id: string
  kind: ToastKind
  message: string
  createdAt: number
}

export const useToastStore = defineStore('toast', {
  state: () => ({
    items: [] as ToastItem[],
  }),
  actions: {
    push(kind: ToastKind, message: string, ttlMs = 2600) {
      const id = crypto.randomUUID ? crypto.randomUUID() : `${Date.now()}-${Math.random()}`
      const item: ToastItem = { id, kind, message, createdAt: Date.now() }
      this.items.push(item)
      window.setTimeout(() => this.remove(id), ttlMs)
      return id
    },
    info(message: string) {
      return this.push('info', message)
    },
    success(message: string) {
      return this.push('success', message)
    },
    error(message: string) {
      return this.push('error', message, 3800)
    },
    remove(id: string) {
      this.items = this.items.filter((x) => x.id !== id)
    },
    clear() {
      this.items = []
    },
  },
})

