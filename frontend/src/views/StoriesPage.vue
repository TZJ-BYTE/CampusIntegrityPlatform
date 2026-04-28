<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import FeedCard from '../components/FeedCard.vue'
import FeedLayout from '../components/FeedLayout.vue'
import coverUrl from '../assets/placeholder-cover.svg'
import { resolveContentAssetUrl } from '../utils/contentAsset'
import {
  contentGetTodayStory,
  contentListStories,
  userIsFavorite,
  userSetFavorite,
  type ApiResponse,
  type StoryDetail,
  type StoryListItem,
} from '../api/tauri'
import { useToastStore } from '../stores/toast'
import { animatePop } from '../utils/motion'

const route = useRoute()
const router = useRouter()
const toast = useToastStore()
const keyword = ref('')
const loading = ref(false)
const today = ref<ApiResponse<StoryDetail> | null>(null)
const list = ref<ApiResponse<{ items: StoryListItem[]; total: number }> | null>(null)
const items = ref<StoryListItem[]>([])
const total = ref(0)
const pageSize = 50
const todayIsFavorite = ref(false)
const todayFavBtnEl = ref<HTMLElement | null>(null)
let debounceTimer: number | null = null

function todayYyyyMMdd() {
  const d = new Date()
  const y = d.getFullYear().toString()
  const m = (d.getMonth() + 1).toString().padStart(2, '0')
  const day = d.getDate().toString().padStart(2, '0')
  return `${y}${m}${day}`
}

async function refresh(reset = true) {
  if (reset) {
    items.value = []
    total.value = 0
  }
  loading.value = true
  try {
    today.value = await contentGetTodayStory({ yyyyMMdd: todayYyyyMMdd() })
    todayIsFavorite.value = false
    if (today.value.ok) {
      const f = await userIsFavorite({ entityType: 'story', entityId: today.value.data.id })
      if (f.ok) todayIsFavorite.value = f.data.isFavorite
    }
    const r = await contentListStories({
      keyword: keyword.value.trim() || undefined,
      limit: pageSize,
      offset: reset ? 0 : items.value.length,
    })
    list.value = r
    if (r.ok) {
      total.value = r.data.total
      items.value = reset ? r.data.items : [...items.value, ...r.data.items]
    }
  } finally {
    loading.value = false
  }
}

const hasMore = computed(() => items.value.length < total.value)

async function loadMore() {
  if (loading.value) return
  if (!hasMore.value) return
  await refresh(false)
}

function openDetail(id: string) {
  router.push({ name: 'storyDetail', params: { id } })
}

async function toggleFavoriteStory(id: string) {
  const current = todayIsFavorite.value
  const next = !current
  const r = await userSetFavorite({ entityType: 'story', entityId: id, isFavorite: next })
  if (r.ok) {
    todayIsFavorite.value = r.data.isFavorite
    if (todayFavBtnEl.value) animatePop(todayFavBtnEl.value)
    toast.success(r.data.isFavorite ? '已收藏' : '已取消收藏')
  } else {
    toast.error(`${r.error.code} · ${r.error.message}`)
  }
}

async function maybeOpenFromQuery() {
  const id = route.query.open
  if (typeof id === 'string' && id.trim()) {
    await router.replace({ name: 'storyDetail', params: { id: id.trim() } })
  }
}

onMounted(async () => {
  await refresh()
  await maybeOpenFromQuery()
})

watch(
  () => route.query.open,
  async () => {
    await maybeOpenFromQuery()
  },
)

watch(keyword, async () => {
  if (debounceTimer != null) window.clearTimeout(debounceTimer)
  debounceTimer = window.setTimeout(() => refresh(true), 450)
})
</script>

<template>
  <div class="page">
    <FeedLayout>
      <template #main>
        <div class="toolbar tute-card">
          <div class="left">
            <input v-model="keyword" class="tute-input" placeholder="搜索标题" @keyup.enter="() => refresh(true)" />
            <button class="tute-btn" type="button" :disabled="loading" @click="() => refresh(true)">
              {{ loading ? '加载中…' : '搜索' }}
            </button>
          </div>
          <div class="right">
            <span v-if="list?.ok" class="tute-muted">已显示 {{ items.length }} / {{ total }} 条</span>
            <span v-else-if="list && !list.ok" class="err">{{ list.error.code }} · {{ list.error.message }}</span>
          </div>
        </div>

        <transition-group name="feed" tag="div" class="feed">
          <FeedCard
            v-if="today?.ok"
            key="today"
            badge="今日推荐"
            :meta="today.data.source ?? '—'"
            :title="today.data.title"
            :subtitle="today.data.dayOfYear ? `第 ${today.data.dayOfYear} 天` : ''"
            :thumb="resolveContentAssetUrl(today.data.coverUrl) ?? coverUrl"
          >
            <div class="clamp3">{{ today.data.body }}</div>
            <template #footer>
              <div class="tags">
                <span class="tute-badge gold">{{ todayIsFavorite ? '已收藏' : '未收藏' }}</span>
              </div>
              <div class="actions">
                <button class="tute-btn-ghost" type="button" @click.stop="openDetail(today.data.id)">打开</button>
                <button ref="todayFavBtnEl" class="tute-btn" type="button" @click.stop="toggleFavoriteStory(today.data.id)">
                  {{ todayIsFavorite ? '取消收藏' : '收藏' }}
                </button>
              </div>
            </template>
          </FeedCard>

          <FeedCard
            v-for="s in items"
            :key="s.id"
            badge="故事"
            :meta="s.source ?? '—'"
            :title="s.title"
            :subtitle="s.dayOfYear ? `第 ${s.dayOfYear} 天` : ''"
            :thumb="resolveContentAssetUrl(s.coverUrl) ?? coverUrl"
            :clickable="true"
            @click="openDetail(s.id)"
          >
            <div class="clamp2">打开后可收藏，并支持复制正文</div>
            <template #footer>
              <div class="tags">
                <span class="tute-badge">{{ s.id }}</span>
              </div>
              <div class="actions">
                <button class="tute-btn-ghost" type="button" @click.stop="openDetail(s.id)">打开</button>
              </div>
            </template>
          </FeedCard>
        </transition-group>

        <div v-if="list?.ok && items.length === 0" class="empty tute-card">暂无数据</div>

        <div v-if="list?.ok && hasMore" class="more">
          <button class="tute-btn-ghost" type="button" :disabled="loading" @click="loadMore">
            {{ loading ? '加载中…' : '加载更多' }}
          </button>
        </div>
      </template>

      <template #aside>
        <div class="side tute-card">
          <div class="sideTitle">学习小贴士</div>
          <div class="sideBody">
            每天读一则故事并做 3 题练习，收藏你觉得“可执行”的做法，长期会更有收益。
          </div>
        </div>
        <div class="side tute-card">
          <div class="sideTitle">提示</div>
          <div class="sideBody">
            <div class="sideRow"><span>建议：</span><span class="tute-muted">收藏用于建立“可复用清单”</span></div>
            <div class="sideRow"><span>动作：</span><span class="tute-muted">阅读抽屉支持复制正文</span></div>
          </div>
        </div>
      </template>
    </FeedLayout>
  </div>
</template>

<style scoped>
.page {
  padding: 0 0 8px;
}

.toolbar {
  padding: 12px 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.left {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  align-items: center;
}

.right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.feed {
  margin-top: 14px;
  display: grid;
  gap: 12px;
}

.tags {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.empty {
  margin-top: 12px;
  padding: 14px 14px;
  color: var(--text-muted);
}

.more {
  margin-top: 12px;
  display: flex;
  justify-content: center;
}

.err {
  font-size: 12px;
  color: #d93026;
}

.actions {
  display: inline-flex;
  gap: 10px;
  align-items: center;
}

.clamp2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.clamp3 {
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.detailMeta {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  align-items: center;
}

.detailBody {
  margin-top: 12px;
  white-space: pre-wrap;
  line-height: 1.8;
  font-size: 12px;
  color: var(--text-secondary);
}

.detailActions {
  margin-top: 14px;
  display: flex;
  gap: 10px;
  justify-content: flex-end;
}

.side {
  padding: 12px 12px;
  border-radius: 10px;
  border: 1px solid rgba(0, 0, 0, 0.06);
}

.sideTitle {
  font-weight: 900;
  font-size: 13px;
}

.sideBody {
  margin-top: 10px;
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.7;
}

.sideRow {
  display: flex;
  gap: 8px;
  margin-top: 6px;
}

.feed-enter-active {
  transition: opacity 180ms ease-out, transform 180ms ease-out;
}
.feed-enter-from {
  opacity: 0;
  transform: translateY(6px);
}

@media (max-width: 980px) {
  .toolbar {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
