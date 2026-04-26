<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useAppStore } from '../stores/app'
import { useProfileStore } from '../stores/profile'
import { useSyncStore } from '../stores/sync'
import { useUiStore } from '../stores/ui'
import AvatarCircle from './AvatarCircle.vue'
import { animateIn } from '../utils/motion'

const app = useAppStore()
const sync = useSyncStore()
const ui = useUiStore()
const profile = useProfileStore()

const open = ref(false)
const rootEl = ref<HTMLElement | null>(null)
const menuEl = ref<HTMLElement | null>(null)

const username = computed(() => (sync.auth?.ok ? (sync.auth.data.username ?? '').trim() : ''))
const deviceId = computed(() => (app.status?.ok ? app.status.data.deviceId : ''))
const nickname = computed(() => profile.profile.nickname.trim())
const avatarSeed = computed(() => username.value || nickname.value || deviceId.value || 'cip')
const avatarLabel = computed(() => nickname.value || username.value || '离线')

function toggle() {
  open.value = !open.value
}

function close() {
  open.value = false
}

function clickOutside(e: MouseEvent) {
  const el = rootEl.value
  if (!el) return
  if (e.target instanceof Node && el.contains(e.target)) return
  close()
}

function onAccount() {
  close()
  ui.open('account')
}

function onProfile() {
  close()
  ui.open('profile')
}

function onSettings() {
  close()
  ui.open('settings')
}

function onDiagnostics() {
  close()
  ui.open('diagnostics')
}

function onAbout() {
  close()
  ui.open('about')
}

async function onLogout() {
  close()
  await sync.logout()
}

const isLoggedIn = computed(() => !!(sync.auth?.ok && sync.auth.data.isLoggedIn))

onMounted(async () => {
  await profile.loadOnce()
  window.addEventListener('click', clickOutside, true)
})

onBeforeUnmount(() => {
  window.removeEventListener('click', clickOutside, true)
})

watch(
  () => open.value,
  async (v) => {
    if (!v) return
    await nextTick()
    if (!menuEl.value) return
    animateIn(menuEl.value, { from: { opacity: 0, y: -8, scale: 0.98 }, to: { opacity: 1, y: 0, scale: 1, duration: 0.24 } })
  },
)
</script>

<template>
  <div ref="rootEl" class="wrap">
    <button class="avatarBtn" type="button" @click="toggle">
      <AvatarCircle
        :size="34"
        :seed="avatarSeed"
        :label="avatarLabel"
        :baseColor="profile.profile.avatarColor"
        :imageDataUrl="profile.profile.avatarImageDataUrl || null"
      />
    </button>

    <div v-if="open" ref="menuEl" class="menu" role="menu">
      <button class="item" type="button" role="menuitem" @click="onAccount">账号与云同步…</button>
      <button class="item" type="button" role="menuitem" @click="onProfile">用户信息…</button>
      <button class="item" type="button" role="menuitem" @click="onSettings">设置…</button>
      <button class="item" type="button" role="menuitem" @click="onDiagnostics">诊断…</button>
      <button class="item" type="button" role="menuitem" @click="onAbout">关于…</button>
      <div class="sep"></div>
      <button v-if="isLoggedIn" class="item danger" type="button" role="menuitem" @click="onLogout">退出登录</button>
    </div>
  </div>
</template>

<style scoped>
.wrap {
  position: relative;
}

.avatarBtn {
  border: 0;
  background: transparent;
  padding: 0;
  cursor: pointer;
  display: grid;
  place-items: center;
}

.menu {
  position: absolute;
  right: 0;
  top: 44px;
  width: 180px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 10px;
  background: #ffffff;
  overflow: hidden;
  box-shadow: 0 18px 60px rgba(0, 0, 0, 0.18);
  z-index: 50;
}

.item {
  width: 100%;
  text-align: left;
  border: 0;
  background: transparent;
  padding: 10px 12px;
  cursor: pointer;
  font-size: 12px;
  color: var(--text-primary);
}

.item:hover {
  background: rgba(0, 0, 0, 0.04);
}

.sep {
  height: 1px;
  background: rgba(0, 0, 0, 0.08);
}

.danger {
  color: #d93026;
}

.danger:hover {
  background: rgba(217, 48, 38, 0.06);
}
</style>
