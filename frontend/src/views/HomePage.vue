<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { useAppStore } from '../stores/app'
import {
  contentGetStory,
  contentGetTodayStory,
  quizGetProgress,
  userListFavorites,
  type ApiResponse,
  type FavoriteItem,
  type QuizProgress,
  type StoryDetail,
} from '../api/tauri'

const app = useAppStore()

const apps = [
  { to: '/venues', label: '廉洁文化场所' },
  { to: '/cases', label: '身边违纪行为警示' },
  { to: '/regulations', label: '政策法规学习' },
  { to: '/stories', label: '每日廉洁故事' },
  { to: '/quiz', label: '廉洁知识竞答' },
  { to: '/favorites', label: '我的收藏' },
]

const slides = [
  { src: '/hero-1.svg', alt: '廉洁文化宣传图 1' },
  { src: '/hero-2.svg', alt: '廉洁文化宣传图 2' },
  { src: '/hero-3.svg', alt: '廉洁文化宣传图 3' },
]

const slideIndex = ref(0)
let timer: number | null = null

function startAuto() {
  stopAuto()
  timer = window.setInterval(() => {
    slideIndex.value = (slideIndex.value + 1) % slides.length
  }, 4200)
}

function stopAuto() {
  if (timer == null) return
  window.clearInterval(timer)
  timer = null
}

function setSlide(i: number) {
  slideIndex.value = i
  startAuto()
}

onMounted(async () => {
  if (!app.status) await app.refreshStatus()
  startAuto()
  await refreshPanels()
})

onUnmounted(() => stopAuto())

const statusLines = computed(() => {
  if (!app.status) return []
  if (!app.status.ok) return [`状态获取失败：${app.status.error.code}`]
  const s = app.status.data
  return [`版本：v${s.appVersion}`, `内容版本：${s.contentVersion}`, `设备标识：${s.deviceId}`]
})

function todayYyyyMMdd() {
  const d = new Date()
  const y = d.getFullYear().toString()
  const m = (d.getMonth() + 1).toString().padStart(2, '0')
  const day = d.getDate().toString().padStart(2, '0')
  return `${y}${m}${day}`
}

const todayStory = ref<ApiResponse<StoryDetail> | null>(null)
const progress = ref<ApiResponse<QuizProgress> | null>(null)
const favorites = ref<ApiResponse<{ items: FavoriteItem[]; total: number }> | null>(null)
const favoriteStoryMap = ref<Record<string, ApiResponse<StoryDetail>>>({})

function favoriteTitle(i: FavoriteItem): string {
  if (i.entityType === 'story') {
    const r = favoriteStoryMap.value[i.entityId]
    if (r && r.ok) return r.data.title
  }
  return `${i.entityType}：${i.entityId}`
}

async function refreshPanels() {
  todayStory.value = await contentGetTodayStory({ yyyyMMdd: todayYyyyMMdd() })
  progress.value = await quizGetProgress()
  favorites.value = await userListFavorites({ limit: 6, offset: 0 })

  const map: Record<string, ApiResponse<StoryDetail>> = {}
  if (favorites.value?.ok) {
    const storyIds = favorites.value.data.items
      .filter((i) => i.entityType === 'story')
      .map((i) => i.entityId)
      .slice(0, 3)
    for (const id of storyIds) {
      map[id] = await contentGetStory({ id })
    }
  }
  favoriteStoryMap.value = map
}
</script>

<template>
  <div class="wrap">
    <div class="hero" @mouseenter="stopAuto" @mouseleave="startAuto">
      <img class="heroImg" :src="slides[slideIndex].src" :alt="slides[slideIndex].alt" />
      <div class="heroDots">
        <button
          v-for="(_, i) in slides"
          :key="i"
          type="button"
          class="dot"
          :class="{ active: i === slideIndex }"
          @click="setSlide(i)"
        />
      </div>
    </div>

    <div class="intro">
      <div class="title">平台概览</div>
      <div class="sub">离线优先 · 数据本地化 · 可复制即用</div>
      <div class="meta">
        <span v-for="l in statusLines" :key="l" class="metaItem">{{ l }}</span>
      </div>
    </div>

    <div class="panels">
      <div class="panel">
        <div class="panelHead">
          <div class="panelTitle">今日推荐</div>
          <RouterLink class="panelLink" to="/stories">更多</RouterLink>
        </div>
        <div v-if="todayStory?.ok" class="panelBody">
          <div class="panelMain">{{ todayStory.data.title }}</div>
          <div class="panelSub clamp2">{{ todayStory.data.body }}</div>
        </div>
        <div v-else class="panelEmpty">暂无推荐内容</div>
      </div>

      <div class="panel">
        <div class="panelHead">
          <div class="panelTitle">学习进度</div>
          <RouterLink class="panelLink" to="/quiz">去答题</RouterLink>
        </div>
        <div v-if="progress?.ok" class="panelBody">
          <div class="panelMain">{{ progress.data.totalPoints }}</div>
          <div class="panelSub">累计积分</div>
        </div>
        <div v-else class="panelEmpty">进度获取失败</div>
      </div>

      <div class="panel">
        <div class="panelHead">
          <div class="panelTitle">最近收藏</div>
          <RouterLink class="panelLink" to="/favorites">查看</RouterLink>
        </div>
        <div v-if="favorites?.ok && favorites.data.items.length" class="favList">
          <div v-for="i in favorites.data.items" :key="`${i.entityType}:${i.entityId}`" class="favItem">
            <div class="favTitle">
              {{ favoriteTitle(i) }}
            </div>
            <div class="favMeta">{{ new Date(i.createdAt).toLocaleString() }}</div>
          </div>
        </div>
        <div v-else class="panelEmpty">暂无收藏</div>
      </div>
    </div>

    <div class="apps">
      <RouterLink v-for="a in apps" :key="a.to" class="appLink tute-link" :to="a.to">
        {{ a.label }}
      </RouterLink>
    </div>
  </div>
</template>

<style scoped>
.wrap {
  display: grid;
  gap: 18px;
}

.hero {
  position: relative;
  border-radius: 10px;
  overflow: hidden;
  border: 1px solid var(--border-color);
  background: #ffffff;
}

.heroImg {
  width: 100%;
  height: 220px;
  display: block;
  object-fit: cover;
}

.heroDots {
  position: absolute;
  right: 10px;
  bottom: 10px;
  display: flex;
  gap: 8px;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  border: 0;
  background: rgba(255, 255, 255, 0.6);
  cursor: pointer;
  transition: all var(--transition);
}

.dot.active {
  width: 18px;
  background: rgba(255, 255, 255, 0.95);
}

.intro {
  padding: 14px 14px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: #ffffff;
}

.title {
  font-size: 16px;
  font-weight: 900;
  color: var(--text-primary);
}

.sub {
  margin-top: 8px;
  font-size: 13px;
  color: var(--text-secondary);
}

.meta {
  margin-top: 10px;
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.metaItem {
  font-size: 12px;
  color: var(--text-muted);
  padding: 4px 8px;
  border-radius: 999px;
  border: 1px solid rgba(0, 0, 0, 0.12);
  background: #ffffff;
}

.panels {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 12px;
}

.panel {
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: #ffffff;
  padding: 12px 12px;
  min-width: 0;
}

.panelHead {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.panelTitle {
  font-weight: 900;
  color: var(--text-primary);
  font-size: 14px;
}

.panelLink {
  text-decoration: none;
  color: var(--primary);
  font-size: 12px;
  transition: all var(--transition);
}

.panelLink:hover {
  text-decoration: underline;
}

.panelBody {
  margin-top: 10px;
}

.panelMain {
  font-size: 18px;
  font-weight: 900;
  color: var(--text-primary);
}

.panelSub {
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.7;
  white-space: pre-wrap;
}

.panelEmpty {
  margin-top: 10px;
  font-size: 12px;
  color: var(--text-muted);
}

.clamp2 {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  overflow: hidden;
}

.favList {
  margin-top: 8px;
  display: grid;
  gap: 10px;
}

.favItem {
  border-top: 1px solid var(--border-color);
  padding-top: 10px;
}

.favItem:first-child {
  border-top: 0;
  padding-top: 0;
}

.favTitle {
  font-size: 13px;
  color: var(--text-primary);
  line-height: 1.4;
}

.favMeta {
  margin-top: 6px;
  font-size: 12px;
  color: var(--text-muted);
}

.apps {
  display: grid;
  grid-template-columns: 1fr 1fr;
  column-gap: 24px;
}

.appLink {
  height: 48px;
  display: flex;
  align-items: center;
  text-decoration: none;
  color: var(--text-primary);
  font-size: 14px;
  border-bottom: 1px solid var(--border-color);
}

@media (max-width: 900px) {
  .heroImg {
    height: 180px;
  }

  .panels {
    grid-template-columns: 1fr;
  }

  .apps {
    grid-template-columns: 1fr;
  }
}
</style>
