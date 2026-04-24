<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { RouterLink, RouterView, useRoute } from 'vue-router'
import { useAppStore } from '../stores/app'

const route = useRoute()
const app = useAppStore()

const pageTitle = computed(() => (route.meta?.title as string) ?? '校园廉洁教育平台')

const nav = [
  { to: '/', label: '概览' },
  { to: '/venues', label: '文化场所' },
  { to: '/cases', label: '案例警示' },
  { to: '/regulations', label: '法规学习' },
  { to: '/stories', label: '每日故事' },
  { to: '/quiz', label: '知识竞答' },
  { to: '/favorites', label: '我的收藏' },
  { to: '/settings', label: '设置' },
]

onMounted(async () => {
  if (!app.status) await app.refreshStatus()
})
</script>

<template>
  <div class="shell">
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
          :class="{ active: i.to === route.path }"
          :to="i.to"
        >
          {{ i.label }}
        </RouterLink>
      </nav>

      <div class="actions">
        <div v-if="app.status?.ok" class="meta">
          <span class="metaItem">v{{ app.status.data.appVersion }}</span>
          <span class="metaSep">|</span>
          <span class="metaItem">内容 {{ app.status.data.contentVersion }}</span>
        </div>
        <button class="btn" type="button" @click="app.refreshStatus">刷新</button>
      </div>
    </header>

    <main class="main">
      <div class="pageHead">
        <div class="pageTitle">{{ pageTitle }}</div>
      </div>
      <section class="body">
        <div class="card">
          <RouterView />
        </div>
      </section>
    </main>
  </div>
</template>

<style scoped>
.shell {
  min-height: 100vh;
  display: grid;
  grid-template-rows: 64px 1fr;
  background: var(--bg-page);
}

.appbar {
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

.meta {
  display: flex;
  align-items: center;
  gap: 8px;
  white-space: nowrap;
}

.metaItem {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.9);
}

.metaSep {
  opacity: 0.85;
}

.btn {
  background: rgba(255, 255, 255, 0.14);
  border: 1px solid rgba(255, 255, 255, 0.22);
  color: var(--white);
  border-radius: 8px;
  padding: 8px 12px;
  cursor: pointer;
  transition: all var(--transition);
}

.btn:hover {
  background: rgba(255, 255, 255, 0.2);
}

.main {
  min-width: 0;
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
