<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import FeedCard from '../components/FeedCard.vue'
import FeedLayout from '../components/FeedLayout.vue'
import coverUrl from '../assets/placeholder-cover.svg'
import { resolveContentAssetUrl } from '../utils/contentAsset'
import {
  contentListRegulations,
  type ApiResponse,
  type RegulationListItem,
} from '../api/tauri'

const route = useRoute()
const router = useRouter()
const keyword = ref('')
const level = ref('')
const loading = ref(false)
const list = ref<ApiResponse<{ items: RegulationListItem[]; total: number }> | null>(null)
const items = ref<RegulationListItem[]>([])
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
    const r = await contentListRegulations({
      keyword: keyword.value.trim() || undefined,
      level: level.value.trim() || undefined,
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
  router.push({ name: 'regulationDetail', params: { id } })
}

async function maybeOpenFromQuery() {
  const id = route.query.open
  if (typeof id === 'string' && id.trim()) {
    await router.replace({ name: 'regulationDetail', params: { id: id.trim() } })
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

watch([keyword, level], async () => {
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
            <input v-model="level" class="tute-input" placeholder="层级（示例：校内制度）" @keyup.enter="() => refresh(true)" />
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
            v-for="r in items"
            :key="r.id"
            :badge="r.level"
            :meta="r.publishedAt ?? '—'"
            :title="r.title"
            :subtitle="r.publishedAt ? `发布日期：${r.publishedAt}` : r.level"
            :thumb="resolveContentAssetUrl(r.coverUrl) ?? coverUrl"
            :clickable="true"
            @click="openDetail(r.id)"
          >
            <div class="clamp2">打开进入学习页：目录、学习资源（视频/图片占位）、复制与收藏。</div>
            <template #footer>
              <div class="tags">
                <span class="tute-badge">{{ r.id }}</span>
              </div>
              <div class="actions">
                <button class="tute-btn-ghost" type="button" @click.stop="openDetail(r.id)">打开</button>
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
          <div class="sideTitle">阅读建议</div>
          <div class="sideBody">
            先看条款目录定位到“你正在做的事”，再对照要点检查流程留痕与公开透明。
          </div>
        </div>
        <div class="side tute-card">
          <div class="sideTitle">筛选提示</div>
          <div class="sideBody">
            <div class="sideRow"><span>层级：</span><span class="tute-muted">如“校内制度 / 学习指引”</span></div>
            <div class="sideRow"><span>关键词：</span><span class="tute-muted">标题支持模糊匹配</span></div>
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

.toc {
  margin-top: 12px;
  padding: 12px 12px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.03);
  border: 1px solid rgba(0, 0, 0, 0.08);
}

.tocTitle {
  font-weight: 900;
  font-size: 13px;
}

.tocList {
  margin-top: 10px;
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.tocBtn {
  border: 1px solid rgba(0, 0, 0, 0.14);
  background: #ffffff;
  border-radius: 999px;
  padding: 6px 10px;
  cursor: pointer;
  font-size: 12px;
  transition: background 160ms ease-out, color 160ms ease-out, border-color 160ms ease-out;
}

.tocBtn:hover {
  border-color: rgba(139, 26, 92, 0.22);
  color: var(--primary);
  background: rgba(139, 26, 92, 0.06);
}

.sections {
  margin-top: 12px;
  display: grid;
  gap: 10px;
}

.sec {
  padding: 12px 12px;
  border-radius: 10px;
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.08);
}

.secTitle {
  font-weight: 800;
  font-size: 13px;
  color: var(--text-primary);
}

.secBody {
  margin-top: 6px;
  white-space: pre-wrap;
  font-size: 13px;
  color: var(--text-primary);
  line-height: 1.7;
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

.emptyInner {
  margin-top: 12px;
  padding: 12px 12px;
  border-radius: 10px;
  border: 1px dashed rgba(0, 0, 0, 0.18);
  color: var(--text-muted);
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
