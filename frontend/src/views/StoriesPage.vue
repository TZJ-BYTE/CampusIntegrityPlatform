<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import {
  contentGetStory,
  contentGetTodayStory,
  contentListStories,
  userSetFavorite,
  type ApiResponse,
  type StoryDetail,
  type StoryListItem,
} from '../api/tauri'

const route = useRoute()
const keyword = ref('')
const loading = ref(false)
const today = ref<ApiResponse<StoryDetail> | null>(null)
const list = ref<ApiResponse<{ items: StoryListItem[]; total: number }> | null>(null)
const selected = ref<ApiResponse<StoryDetail> | null>(null)

function todayYyyyMMdd() {
  const d = new Date()
  const y = d.getFullYear().toString()
  const m = (d.getMonth() + 1).toString().padStart(2, '0')
  const day = d.getDate().toString().padStart(2, '0')
  return `${y}${m}${day}`
}

async function refresh() {
  loading.value = true
  try {
    today.value = await contentGetTodayStory({ yyyyMMdd: todayYyyyMMdd() })
    list.value = await contentListStories({
      keyword: keyword.value.trim() || undefined,
      limit: 50,
      offset: 0,
    })
  } finally {
    loading.value = false
  }
}

async function openDetail(id: string) {
  selected.value = await contentGetStory({ id })
}

async function favoriteStory(id: string) {
  await userSetFavorite({ entityType: 'story', entityId: id, isFavorite: true })
}

async function maybeOpenFromQuery() {
  const id = route.query.open
  if (typeof id === 'string' && id.trim()) {
    await openDetail(id)
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
</script>

<template>
  <div>
    <div class="toolbar tute-card">
      <div class="left">
        <input v-model="keyword" class="tute-input" placeholder="搜索标题" @keyup.enter="refresh" />
        <button class="tute-btn" type="button" :disabled="loading" @click="refresh">
          {{ loading ? '加载中…' : '搜索' }}
        </button>
      </div>
      <div class="right">
        <span v-if="list?.ok" class="tute-muted">历史故事：{{ list.data.total }} 条</span>
      </div>
    </div>

    <div v-if="today?.ok" class="today">
      <div class="todayTitle">今日推荐 · {{ today.data.title }}</div>
      <div class="todayBody">{{ today.data.body }}</div>
      <div class="todayActions">
        <button class="tute-btn" type="button" @click="favoriteStory(today.data.id)">收藏</button>
        <button class="tute-btn-ghost" type="button" @click="openDetail(today.data.id)">查看详情</button>
      </div>
    </div>
    <div v-else-if="today && !today.ok" class="err">{{ today.error.code }} · {{ today.error.message }}</div>

    <div v-else-if="list && !list.ok" class="err">{{ list.error.code }} · {{ list.error.message }}</div>

    <div v-if="list?.ok" class="tute-grid grid">
      <button v-for="s in list.data.items" :key="s.id" class="tute-grid-card item" type="button" @click="openDetail(s.id)">
        <div class="title">{{ s.title }}</div>
        <div class="sub">{{ s.source ?? '—' }}<span v-if="s.dayOfYear"> · 第 {{ s.dayOfYear }} 天</span></div>
        <div class="tags">
          <span class="tute-badge">{{ s.id }}</span>
        </div>
      </button>
      <div v-if="list.data.items.length === 0" class="empty tute-card">暂无数据</div>
    </div>

    <div v-if="selected" class="modalBackdrop" @click.self="selected = null">
      <div class="modal">
        <div v-if="selected.ok" class="modalBody">
          <div class="modalHeader">
            <div class="modalTitle">{{ selected.data.title }}</div>
            <div class="modalTags">
              <span class="tute-badge gold">{{ selected.data.source ?? '—' }}</span>
              <span class="tute-badge">{{ selected.data.id }}</span>
            </div>
          </div>
          <div class="section">
            <div class="k">正文</div>
            <div class="v">{{ selected.data.body }}</div>
          </div>
          <div class="modalActions">
            <button class="tute-btn" type="button" @click="favoriteStory(selected.data.id)">收藏</button>
            <button class="tute-btn-ghost" type="button" @click="selected = null">关闭</button>
          </div>
        </div>
        <div v-else class="modalBody">
          <div class="err">{{ selected.error.code }} · {{ selected.error.message }}</div>
          <div class="modalActions">
            <button class="tute-btn-ghost" type="button" @click="selected = null">关闭</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
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
  font-size: 12px;
  color: var(--text-muted);
}

.err {
  margin-top: 12px;
  font-size: 12px;
  color: #d93026;
}

.today {
  margin-top: 16px;
  padding: 14px 14px;
  border-radius: 4px;
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.08);
  box-shadow: var(--shadow-card);
}

.todayTitle {
  font-weight: 900;
  font-size: 13px;
}

.todayBody {
  margin-top: 10px;
  font-size: 13px;
  color: var(--text-primary);
  line-height: 1.8;
  white-space: pre-wrap;
}

.todayActions {
  margin-top: 12px;
  display: flex;
  gap: 10px;
  justify-content: flex-end;
}

.grid {
  margin-top: 16px;
}

.item {
  text-align: left;
  border: 0;
}

.title {
  font-weight: 800;
  font-size: 13px;
}

.sub {
  margin-top: 8px;
  font-size: 13px;
  color: var(--text-muted);
}

.tags {
  margin-top: 10px;
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.empty {
  padding: 14px;
  color: var(--text-muted);
  grid-column: 1 / -1;
}

.modalBackdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  display: grid;
  place-items: center;
  padding: 16px;
}

.modal {
  width: min(880px, 96vw);
  max-height: 88vh;
  overflow: auto;
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.16);
  border-radius: 6px;
  box-shadow: var(--shadow-card-hover);
}

.modalBody {
  padding: 14px 14px;
}

.modalHeader {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: flex-start;
}

.modalTitle {
  font-weight: 900;
  font-size: 14px;
}

.modalTags {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.section {
  margin-top: 12px;
  padding: 12px 12px;
  border-radius: 4px;
  background: #fafafa;
  border: 1px solid rgba(0, 0, 0, 0.08);
}

.k {
  font-weight: 800;
  font-size: 12px;
}

.v {
  margin-top: 8px;
  font-size: 13px;
  color: var(--text-primary);
  line-height: 1.7;
  white-space: pre-wrap;
}

.modalActions {
  margin-top: 12px;
  display: flex;
  gap: 10px;
  justify-content: flex-end;
}

@media (max-width: 980px) {
  .toolbar {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
