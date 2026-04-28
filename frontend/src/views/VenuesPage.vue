<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import FeedCard from '../components/FeedCard.vue'
import FeedLayout from '../components/FeedLayout.vue'
import coverUrl from '../assets/placeholder-cover.svg'
import { resolveContentAssetUrl } from '../utils/contentAsset'
import {
  contentListVenues,
  type ApiResponse,
  type VenueListItem,
} from '../api/tauri'

const route = useRoute()
const router = useRouter()
const keyword = ref('')
const typeFilter = ref('')
const loading = ref(false)
const result = ref<ApiResponse<{ items: VenueListItem[]; total: number }> | null>(null)
const items = ref<VenueListItem[]>([])
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
    const r = await contentListVenues({
      keyword: keyword.value.trim() || undefined,
      type: typeFilter.value.trim() || undefined,
      limit: pageSize,
      offset: reset ? 0 : items.value.length,
    })
    result.value = r
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
  router.push({ name: 'venueDetail', params: { id } })
}

async function maybeOpenFromQuery() {
  const id = route.query.open
  if (typeof id === 'string' && id.trim()) {
    await router.replace({ name: 'venueDetail', params: { id: id.trim() } })
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

watch([keyword, typeFilter], async () => {
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
            <input v-model="keyword" class="tute-input" placeholder="搜索名称（示例：廉洁）" @keyup.enter="() => refresh(true)" />
            <input v-model="typeFilter" class="tute-input" placeholder="类型（示例：文化展示）" @keyup.enter="() => refresh(true)" />
            <button class="tute-btn" type="button" :disabled="loading" @click="() => refresh(true)">
              {{ loading ? '加载中…' : '搜索' }}
            </button>
          </div>
          <div class="right">
            <span v-if="result?.ok" class="tute-muted">已显示 {{ items.length }} / {{ total }} 条</span>
            <span v-else-if="result && !result.ok" class="err">{{ result.error.code }} · {{ result.error.message }}</span>
          </div>
        </div>

        <transition-group name="feed" tag="div" class="feed">
          <FeedCard
            v-for="v in items"
            :key="v.id"
            :badge="v.type"
            meta="场所"
            :title="v.name"
            :subtitle="v.type"
            :thumb="resolveContentAssetUrl(v.coverUrl) ?? coverUrl"
            :clickable="true"
            @click="openDetail(v.id)"
          >
            <div class="clamp2">打开后可查看简介、开放时间与联系方式</div>
            <template #footer>
              <div class="tags">
                <span class="tute-badge">{{ v.id }}</span>
              </div>
              <div class="actions">
                <button class="tute-btn-ghost" type="button" @click.stop="openDetail(v.id)">打开</button>
              </div>
            </template>
          </FeedCard>
        </transition-group>

        <div v-if="result?.ok && items.length === 0" class="empty tute-card">暂无数据</div>

        <div v-if="result?.ok && hasMore" class="more">
          <button class="tute-btn-ghost" type="button" :disabled="loading" @click="loadMore">
            {{ loading ? '加载中…' : '加载更多' }}
          </button>
        </div>
      </template>

      <template #aside>
        <div class="side tute-card">
          <div class="sideTitle">探索建议</div>
          <div class="sideBody">优先收藏你计划参观或组织活动的点位，后续可作为线下任务清单。</div>
        </div>
        <div class="side tute-card">
          <div class="sideTitle">筛选提示</div>
          <div class="sideBody">
            <div class="sideRow"><span>类型：</span><span class="tute-muted">文化展示 / 红色教育 / 实践体验</span></div>
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

.clamp2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
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
  font-size: 12px;
  font-weight: 900;
}

.v {
  margin-top: 8px;
  font-size: 13px;
  color: var(--text-primary);
  line-height: 1.8;
  white-space: pre-wrap;
}

.detailActions {
  margin-top: 14px;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
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
