import { defineStore } from 'pinia'
import { appGetStatus, type ApiResponse, type AppStatus } from '../api/tauri'

export const useAppStore = defineStore('app', {
  state: () => ({
    status: null as ApiResponse<AppStatus> | null,
  }),
  actions: {
    async refreshStatus() {
      this.status = await appGetStatus()
    },
  },
})

