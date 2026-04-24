<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import {
  contentGetCase,
  contentListCases,
  userSetFavorite,
  type ApiResponse,
  type CaseDetail,
  type CaseListItem,
} from '../api/tauri'

const route = useRoute()
const keyword = ref('')
const scene = ref('')
const loading = ref(false)
const list = ref<ApiResponse<{ items: CaseListItem[]; total: number }> | null>(null)
const selected = ref<ApiResponse<CaseDetail> | null>(null)

async function refresh() {
  loading.value = true
  try {
    list.value = await contentListCases({
      keyword: keyword.value.trim() || undefined,
      scene: scene.value.trim() || undefined,
      limit: 50,
      offset: 0,
    })
  } finally {
    loading.value = false
  }
}

async function openDetail(id: string) {
  selected.value = await contentGetCase({ id })
}

async function favoriteCurrent() {
  if (!selected.value?.ok) return
  await userSetFavorite({ entityType: 'case', entityId: selected.value.data.id, isFavorite: true })
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
        <input v-model="keyword" class="tute-input" placeholder="关键词（标题/摘要）" @keyup.enter="refresh" />
        <input v-model="scene" class="tute-input" placeholder="场景（示例：班级管理）" @keyup.enter="refresh" />
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
      <button v-for="c in list.data.items" :key="c.id" class="tute-grid-card item" type="button" @click="openDetail(c.id)">
        <div class="title">{{ c.title }}</div>
        <div class="sub">{{ c.summary }}</div>
        <div class="tags">
          <span class="tute-badge gold">{{ c.scene }}</span>
          <span class="tute-badge">{{ c.id }}</span>
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
              <span class="tute-badge gold">{{ selected.data.scene }}</span>
              <span class="tute-badge">{{ selected.data.id }}</span>
            </div>
          </div>
          <div class="section">
            <div class="k">情景描述</div>
            <div class="v">{{ selected.data.body }}</div>
          </div>
          <div v-if="selected.data.violation" class="section">
            <div class="k">违纪风险</div>
            <div class="v">{{ selected.data.violation }}</div>
          </div>
          <div v-if="selected.data.correctAction" class="section">
            <div class="k">正确做法</div>
            <div class="v">{{ selected.data.correctAction }}</div>
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
}

.title {
  font-weight: 800;
  font-size: 14px;
}

.sub {
  margin-top: 8px;
  font-size: 13px;
  color: var(--text-muted);
  line-height: 1.7;
}

.tags {
  margin-top: 10px;
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.empty {
  padding: 14px;
  grid-column: 1 / -1;
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
  width: min(860px, 96vw);
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
