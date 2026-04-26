<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { RouterLink, RouterView, useRoute } from 'vue-router'
import { useAppStore } from '../stores/app'
import { useContentUpdateStore } from '../stores/contentUpdate'
import { useSyncStore } from '../stores/sync'
import { useUiStore } from '../stores/ui'
import AboutModal from './AboutModal.vue'
import AccountSheet from './AccountSheet.vue'
import AuthModal from './AuthModal.vue'
import DiagnosticsModal from './DiagnosticsModal.vue'
import ProfileModal from './ProfileModal.vue'
import SettingsModal from './SettingsModal.vue'
import ToastHost from './ToastHost.vue'
import UserMenu from './UserMenu.vue'
import WelcomeModal from './WelcomeModal.vue'

const route = useRoute()
const app = useAppStore()
const sync = useSyncStore()
const contentUpdate = useContentUpdateStore()
const ui = useUiStore()

const pageTitle = computed(() => (route.meta?.title as string) ?? '校园廉洁教育平台')
const syncOnlineLabel = computed(() => {
  if (!sync.auth?.ok) return '云 —'
  if (!sync.auth.data.isLoggedIn) return '云 未连接'
  if (sync.busy) return '云 同步中'
  if (sync.lastErrorAt && (!sync.lastOkAt || sync.lastErrorAt > sync.lastOkAt)) return '云 异常'
  return '云 已连接'
})
const contentUpdateLabel = computed(() => (contentUpdate.busy ? '内容更新中' : null))

const nav = [
  { to: '/', label: '概览' },
  { to: '/venues', label: '文化场所' },
  { to: '/cases', label: '案例警示' },
  { to: '/regulations', label: '法规学习' },
  { to: '/stories', label: '每日故事' },
  { to: '/quiz', label: '知识竞答' },
  { to: '/favorites', label: '我的收藏' },
]

function isActiveNav(to: string) {
  if (to === '/') return route.path === '/'
  return route.path === to || route.path.startsWith(`${to}/`)
}

onMounted(async () => {
  if (!app.status) await app.refreshStatus()
  try {
    const onboarded = localStorage.getItem('cip:onboarded') === '1'
    if (!onboarded && !ui.overlay) ui.open('onboarding')
  } catch {
    if (!ui.overlay) ui.open('onboarding')
  }
})
</script>

<template>
  <div class="shell">
    <WelcomeModal />
    <AuthModal />
    <AccountSheet />
    <SettingsModal />
    <ProfileModal />
    <DiagnosticsModal />
    <AboutModal />
    <ToastHost />
    <header class="appbar">
      <div class="brand">
        <div class="logo">廉</div>
        <div class="brandText">
          <div class="brandName">廉洁教育平台</div>
          <div class="brandSub">离线优先</div>
        </div>
      </div>

      <nav class="nav">
        <RouterLink
          v-for="i in nav"
          :key="i.to"
          class="navItem"
          :class="{ active: isActiveNav(i.to) }"
          :to="i.to"
        >
          {{ i.label }}
        </RouterLink>
      </nav>

      <div class="actions">
        <div class="cloud">
          <span class="cloudItem">{{ syncOnlineLabel }}</span>
          <span v-if="contentUpdateLabel" class="cloudSep">·</span>
          <span v-if="contentUpdateLabel" class="cloudItem">{{ contentUpdateLabel }}</span>
        </div>
        <UserMenu />
      </div>
    </header>

    <main class="main">
      <div class="pageHead">
        <div class="pageTitle">{{ pageTitle }}</div>
      </div>
      <section class="body">
        <div class="card">
          <RouterView v-slot="{ Component, route: r }">
            <Transition name="page" mode="out-in">
              <component :is="Component" :key="r.fullPath" />
            </Transition>
          </RouterView>
        </div>
      </section>
    </main>
  </div>
</template>

<style scoped>
.shell {
  height: 100vh;
  display: grid;
  grid-template-rows: auto 1fr;
  background: var(--bg-page);
}

.appbar {
  position: sticky;
  top: 0;
  z-index: 1200;
  background: var(--primary);
  color: var(--white);
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 0 16px;
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 180px;
}

.logo {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.14);
  border: 1px solid rgba(255, 255, 255, 0.22);
  display: grid;
  place-items: center;
  font-weight: 900;
}

.brandName {
  font-weight: 900;
  color: var(--white);
  font-size: 13px;
}

.brandSub {
  margin-top: 4px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.85);
}

.nav {
  display: flex;
  align-items: center;
  gap: 2px;
  height: 64px;
  overflow: auto;
  flex: 1;
  min-width: 0;
}

.navItem {
  text-decoration: none;
  color: rgba(255, 255, 255, 0.92);
  padding: 0 12px;
  height: 44px;
  display: flex;
  align-items: center;
  border-radius: 8px;
  transition: all var(--transition);
  white-space: nowrap;
}

.navItem:hover {
  background: rgba(255, 255, 255, 0.12);
  color: #ffffff;
}

.navItem.active {
  background: rgba(255, 255, 255, 0.18);
  color: #ffffff;
  font-weight: 900;
}

.actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.cloud {
  display: flex;
  align-items: center;
  gap: 6px;
  white-space: nowrap;
}

.cloudItem {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.9);
}

.cloudSep {
  opacity: 0.9;
}

.main {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.pageHead {
  padding: 14px 16px 0;
}

.pageTitle {
  font-weight: 900;
  font-size: 16px;
  color: var(--text-primary);
}

.body {
  padding: 12px 16px 16px;
  overflow: auto;
  flex: 1;
  min-height: 0;
}

.card {
  background: #ffffff;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 16px;
}

@media (max-width: 980px) {
  .appbar {
    flex-wrap: wrap;
    height: auto;
    padding: 10px 12px;
    gap: 10px;
  }

  .brand {
    min-width: unset;
  }

  .nav {
    width: 100%;
    height: auto;
  }

  .actions {
    width: 100%;
    justify-content: flex-end;
  }
}
</style>
