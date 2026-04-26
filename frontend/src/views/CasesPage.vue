<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import FeedCard from '../components/FeedCard.vue'
import FeedLayout from '../components/FeedLayout.vue'
import {
  contentListCases,
  type ApiResponse,
  type CaseListItem,
} from '../api/tauri'

const route = useRoute()
const router = useRouter()
const keyword = ref('')
const scene = ref('')
const loading = ref(false)
const list = ref<ApiResponse<{ items: CaseListItem[]; total: number }> | null>(null)
const items = ref<CaseListItem[]>([])
const total = ref(0)
const pageSize = 50
let debounceTimer: number | null = null

async function refresh(reset = true) {
  if (reset) {
    items.value = []
    total.value = 0
  }
  loading.value = true
  try {
    const r = await contentListCases({
      keyword: keyword.value.trim() || undefined,
      scene: scene.value.trim() || undefined,
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
  router.push({ name: 'caseDetail', params: { id } })
}

async function maybeOpenFromQuery() {
  const id = route.query.open
  if (typeof id === 'string' && id.trim()) {
    await router.replace({ name: 'caseDetail', params: { id: id.trim() } })
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

watch([keyword, scene], async () => {
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
            <input v-model="keyword" class="tute-input" placeholder="关键词（标题/摘要）" @keyup.enter="() => refresh(true)" />
            <input v-model="scene" class="tute-input" placeholder="场景（示例：班级管理）" @keyup.enter="() => refresh(true)" />
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
            v-for="c in items"
            :key="c.id"
            badge="案例"
            :meta="c.scene"
            :title="c.title"
            :subtitle="c.summary"
            :clickable="true"
            @click="openDetail(c.id)"
          >
            <template #footer>
              <div class="tags">
                <span class="tute-badge gold">{{ c.scene }}</span>
                <span class="tute-badge">{{ c.id }}</span>
              </div>
              <div class="actions">
                <button class="tute-btn-ghost" type="button" @click.stop="openDetail(c.id)">打开</button>
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
          <div class="sideTitle">使用建议</div>
          <div class="sideBody">
            先收藏你觉得可复用的“正确做法”，再把要点转成自己的检查清单。
          </div>
        </div>
        <div class="side tute-card">
          <div class="sideTitle">常用场景</div>
          <div class="sideBody">
            <div class="sideRow"><span>示例：</span><span class="tute-muted">班级管理 / 评优评先 / 考试管理</span></div>
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

.err {
  color: #d93026;
  font-size: 12px;
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

.actions {
  display: inline-flex;
  gap: 10px;
  align-items: center;
}

.detailMeta {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  align-items: center;
}

.block {
  margin-top: 12px;
  padding: 12px 12px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.03);
  border: 1px solid rgba(0, 0, 0, 0.08);
}

.k {
  font-weight: 900;
  font-size: 12px;
}

.v {
  margin-top: 8px;
  font-size: 13px;
  color: var(--text-primary);
  line-height: 1.7;
  white-space: pre-wrap;
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
