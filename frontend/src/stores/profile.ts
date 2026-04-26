import { defineStore } from 'pinia'
import { userGetProfile, userUpdateProfile, type UserProfile } from '../api/tauri'

export const useProfileStore = defineStore('profile', {
  state: () => ({
    profile: { nickname: '', avatarColor: '', avatarImageDataUrl: '' } as UserProfile,
    _loaded: false,
  }),
  actions: {
    async loadOnce() {
      if (this._loaded) return
      this._loaded = true
      const r = await userGetProfile()
      if (r.ok) this.profile = r.data
    },
    async setNickname(nickname: string) {
      this.profile.nickname = nickname
      const r = await userUpdateProfile({ nickname })
      if (r.ok) this.profile = r.data
    },
    async setAvatarColor(avatarColor: string) {
      this.profile.avatarColor = avatarColor
      const r = await userUpdateProfile({ avatarColor })
      if (r.ok) this.profile = r.data
    },
    async setAvatarImageDataUrl(avatarImageDataUrl: string) {
      this.profile.avatarImageDataUrl = avatarImageDataUrl
      const r = await userUpdateProfile({ avatarImageDataUrl })
      if (r.ok) this.profile = r.data
    },
    async clearAvatarImage() {
      this.profile.avatarImageDataUrl = ''
      const r = await userUpdateProfile({ avatarImageDataUrl: '' })
      if (r.ok) this.profile = r.data
    },
  },
})
