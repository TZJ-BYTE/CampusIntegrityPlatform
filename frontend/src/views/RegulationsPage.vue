<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import {
  contentGetRegulation,
  contentListRegulations,
  userSetFavorite,
  type ApiResponse,
  type RegulationDetail,
  type RegulationListItem,
} from '../api/tauri'

const route = useRoute()
const keyword = ref('')
const level = ref('')
const loading = ref(false)
const list = ref<ApiResponse<{ items: RegulationListItem[]; total: number }> | null>(null)
const selected = ref<ApiResponse<RegulationDetail> | null>(null)

async function refresh() {
  loading.value = true
  try {
    list.value = await contentListRegulations({
      keyword: keyword.value.trim() || undefined,
      level: level.value.trim() || undefined,
      limit: 50,
      offset: 0,
    })
  } finally {
    loading.value = false
  }
}

async function openDetail(id: string) {
  selected.value = await contentGetRegulation({ id })
}

async function favoriteCurrent() {
  if (!selected.value?.ok) return
  await userSetFavorite({ entityType: 'regulation', entityId: selected.value.data.id, isFavorite: true })
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
        <input v-model="level" class="tute-input" placeholder="层级（示例：校内制度）" @keyup.enter="refresh" />
        <button class="tute-btn" type="button" :disabled="loading" @click="refresh">
          {{ loading ? '加载中…' : '搜索' }}
        </button>
      </div>
      <div class="right">
        <span v-if="list?.ok" class="tute-muted">共 {{ list.data.total }} 条</span>
        <span v-else-if="list && !list.ok" class="err">{{ list.error.code }} · {{ list.error.message }}</span>
      </div>
    </div>

    <div v-if="list?.ok" class="tute-grid grid">
      <button v-for="r in list.data.items" :key="r.id" class="tute-grid-card item" type="button" @click="openDetail(r.id)">
        <div class="title">{{ r.title }}</div>
        <div class="sub">{{ r.level }}<span v-if="r.publishedAt"> · {{ r.publishedAt }}</span></div>
        <div class="tags">
          <span class="tute-badge">{{ r.id }}</span>
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
              <span class="tute-badge gold">{{ selected.data.level }}</span>
              <span class="tute-badge">{{ selected.data.id }}</span>
            </div>
          </div>
          <div class="section">
            <div class="k">来源</div>
            <div class="v">{{ selected.data.source ?? '—' }}</div>
          </div>
          <div v-if="selected.data.sections.length > 0" class="section">
            <div class="k">条款</div>
            <div class="v">
              <div v-for="s in selected.data.sections" :key="s.id" class="sec">
                <div class="secTitle">
                  {{ s.chapter ?? '' }} {{ s.articleNo ?? '' }} {{ s.title ?? '' }}
                </div>
                <div class="secBody">{{ s.body }}</div>
              </div>
            </div>
          </div>
          <div v-else class="section">
            <div class="k">条款</div>
            <div class="v">暂无条款</div>
          </div>
          <div class="modalActions">
            <button class="tute-btn" type="button" @click="favoriteCurrent">收藏</button>
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
  color: #d93026;
  font-size: 12px;
}

.grid {
  margin-top: 16px;
}

.item {
  text-align: left;
  border: 0;
  width: 100%;
}

.title {
  font-weight: 800;
  font-size: 14px;
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
  grid-column: 1 / -1;
  padding: 14px;
  color: var(--text-muted);
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
  width: min(900px, 96vw);
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
}

.sec {
  padding: 10px 10px;
  border-radius: 4px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  background: #ffffff;
  margin-top: 8px;
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
