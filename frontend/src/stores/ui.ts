import { defineStore } from 'pinia'

export type OverlayName = 'onboarding' | 'auth' | 'account' | 'settings' | 'profile' | 'diagnostics' | 'about'

export const useUiStore = defineStore('ui', {
  state: () => ({
    overlay: null as OverlayName | null,
  }),
  actions: {
    open(name: OverlayName) {
      this.overlay = name
    },
    close() {
      this.overlay = null
    },
  },
})
