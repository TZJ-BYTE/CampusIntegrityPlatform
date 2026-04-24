<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import {
  contentGetVenue,
  contentListVenues,
  userSetFavorite,
  type ApiResponse,
  type VenueDetail,
  type VenueListItem,
} from '../api/tauri'

const route = useRoute()
const keyword = ref('')
const typeFilter = ref('')
const loading = ref(false)
const result = ref<ApiResponse<{ items: VenueListItem[]; total: number }> | null>(null)
const selected = ref<ApiResponse<VenueDetail> | null>(null)

async function refresh() {
  loading.value = true
  try {
    result.value = await contentListVenues({
      keyword: keyword.value.trim() || undefined,
      type: typeFilter.value.trim() || undefined,
      limit: 50,
      offset: 0,
    })
  } finally {
    loading.value = false
  }
}

async function openDetail(id: string) {
  selected.value = await contentGetVenue({ id })
}

async function favoriteCurrent() {
  if (!selected.value?.ok) return
  await userSetFavorite({ entityType: 'venue', entityId: selected.value.data.id, isFavorite: true })
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
        <input v-model="keyword" class="tute-input" placeholder="搜索名称（示例：廉洁）" @keyup.enter="refresh" />
        <input v-model="typeFilter" class="tute-input" placeholder="类型（示例：文化展示）" @keyup.enter="refresh" />
        <button class="tute-btn" type="button" :disabled="loading" @click="refresh">
          {{ loading ? '加载中…' : '搜索' }}
        </button>
      </div>
      <div class="right">
        <span v-if="result?.ok" class="tute-muted">共 {{ result.data.total }} 条</span>
        <span v-else-if="result && !result.ok" class="err">{{ result.error.code }} · {{ result.error.message }}</span>
      </div>
    </div>

    <div v-if="result?.ok" class="tute-grid grid">
      <button v-for="v in result.data.items" :key="v.id" class="tute-grid-card item" type="button" @click="openDetail(v.id)">
        <div class="icon"></div>
        <div class="name">{{ v.name }}</div>
        <div class="meta">
          <span class="tute-badge gold">{{ v.type }}</span>
          <span class="tute-badge">{{ v.id }}</span>
        </div>
      </button>
      <div v-if="result.data.items.length === 0" class="empty tute-card">暂无数据</div>
    </div>

    <div v-if="selected" class="modalBackdrop" @click.self="selected = null">
      <div class="modal">
        <div v-if="selected.ok" class="modalBody">
          <div class="modalHeader">
            <div class="modalTitle">{{ selected.data.name }}</div>
            <div class="modalTags">
              <span class="tute-badge gold">{{ selected.data.type }}</span>
              <span class="tute-badge">{{ selected.data.id }}</span>
            </div>
          </div>

          <div v-if="selected.data.location" class="section">
            <div class="k">地点</div>
            <div class="v">{{ selected.data.location }}</div>
          </div>
          <div v-if="selected.data.openHours" class="section">
            <div class="k">开放时间</div>
            <div class="v">{{ selected.data.openHours }}</div>
          </div>
          <div v-if="selected.data.contact" class="section">
            <div class="k">联系方式</div>
            <div class="v">{{ selected.data.contact }}</div>
          </div>
          <div v-if="selected.data.description" class="section">
            <div class="k">简介</div>
            <div class="v">{{ selected.data.description }}</div>
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
}

.grid {
  margin-top: 16px;
}

.item {
  text-align: left;
  border: 0;
}

.icon {
  width: 34px;
  height: 34px;
  border-radius: 50%;
  background: rgba(139, 26, 92, 0.12);
  border: 1px solid rgba(139, 26, 92, 0.2);
}

.name {
  margin-top: 10px;
  font-size: 14px;
  font-weight: 800;
}

.meta {
  margin-top: 10px;
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.empty {
  grid-column: 1 / -1;
  padding: 14px 14px;
  color: var(--text-muted);
  box-shadow: var(--shadow-card);
  border-radius: 4px;
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
  font-size: 12px;
  font-weight: 800;
}

.v {
  margin-top: 8px;
  font-size: 13px;
  color: var(--text-primary);
  line-height: 1.8;
  white-space: pre-wrap;
}

.modalActions {
  margin-top: 12px;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
