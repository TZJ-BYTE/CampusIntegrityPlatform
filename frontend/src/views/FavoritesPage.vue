<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  contentResolveEntities,
  userListFavorites,
  userSetFavorite,
  type ApiResponse,
  type FavoriteItem,
  type ResolvedEntity,
} from '../api/tauri'

const router = useRouter()

const loading = ref(false)
const list = ref<ApiResponse<{ items: FavoriteItem[]; total: number }> | null>(null)
const resolved = ref<ApiResponse<{ items: ResolvedEntity[] }> | null>(null)
const message = ref('')

function setMessage(s: string) {
  message.value = s
}

function keyOf(i: { entityType: string; entityId: string }) {
  return `${i.entityType}:${i.entityId}`
}

const rmap = computed(() => {
  const map: Record<string, ResolvedEntity> = {}
  if (resolved.value?.ok) {
    for (const r of resolved.value.data.items) {
      map[keyOf(r)] = r
    }
  }
  return map
})

function formatTime(ms: number) {
  return new Date(ms).toLocaleString()
}

function targetPath(entityType: string) {
  if (entityType === 'case') return '/cases'
  if (entityType === 'regulation') return '/regulations'
  if (entityType === 'story') return '/stories'
  if (entityType === 'venue') return '/venues'
  return null
}

async function refresh() {
  loading.value = true
  try {
    list.value = await userListFavorites({ limit: 200, offset: 0 })
    if (list.value.ok) {
      resolved.value = await contentResolveEntities({
        items: list.value.data.items.map((i) => ({ entityType: i.entityType, entityId: i.entityId })),
      })
    } else {
      resolved.value = null
    }
  } finally {
    loading.value = false
  }
}

async function open(i: FavoriteItem) {
  const p = targetPath(i.entityType)
  if (!p) {
    setMessage(`暂不支持打开：${i.entityType}`)
    return
  }
  await router.push({ path: p, query: { open: i.entityId } })
}

async function remove(i: FavoriteItem) {
  const ok = window.confirm('确认取消收藏？')
  if (!ok) return
  const r = await userSetFavorite({ entityType: i.entityType, entityId: i.entityId, isFavorite: false })
  if (r.ok) {
    setMessage('已取消收藏')
    await refresh()
  } else {
    setMessage(`${r.error.code} · ${r.error.message}`)
  }
}

onMounted(refresh)
</script>

<template>
  <div>
    <div class="toolbar tute-card">
      <div class="left">
        <button class="tute-btn" type="button" :disabled="loading" @click="refresh">
          {{ loading ? '加载中…' : '刷新' }}
        </button>
      </div>
      <div class="right">
        <span v-if="list?.ok" class="tute-muted">共 {{ list.data.total }} 条</span>
        <span v-else-if="list && !list.ok" class="err">{{ list.error.code }} · {{ list.error.message }}</span>
      </div>
    </div>

    <div v-if="message" class="message">{{ message }}</div>

    <div v-if="list?.ok" class="list">
      <div v-if="list.data.items.length === 0" class="empty tute-card">暂无收藏</div>

      <div v-else class="items">
        <div v-for="i in list.data.items" :key="keyOf(i)" class="item">
          <div class="main">
            <div class="title">
              {{
                rmap[keyOf(i)]?.title ??
                  (rmap[keyOf(i)]?.exists === false ? `（已不存在）${i.entityType}：${i.entityId}` : `${i.entityType}：${i.entityId}`)
              }}
            </div>
            <div class="sub">
              <span v-if="rmap[keyOf(i)]?.subtitle">{{ rmap[keyOf(i)]?.subtitle }}</span>
              <span v-else class="tute-muted">{{ i.entityType }}</span>
              <span class="sep">·</span>
              <span class="tute-muted">{{ formatTime(i.createdAt) }}</span>
            </div>
          </div>
          <div class="actions">
            <button class="tute-btn-ghost" type="button" @click="open(i)">打开</button>
            <button class="tute-btn-ghost danger" type="button" @click="remove(i)">取消收藏</button>
          </div>
        </div>
      </div>
    </div>

    <div v-else-if="list && !list.ok" class="err">{{ list.error.code }} · {{ list.error.message }}</div>
    <div v-else class="tute-muted">加载中…</div>
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

.message {
  margin-top: 12px;
  padding: 10px 12px;
  border-radius: 6px;
  background: rgba(139, 26, 92, 0.06);
  border: 1px solid rgba(139, 26, 92, 0.18);
  color: var(--text-secondary);
  font-size: 12px;
}

.list {
  margin-top: 16px;
}

.items {
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 6px;
  overflow: hidden;
  background: #ffffff;
}

.item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 12px;
  border-top: 1px solid rgba(0, 0, 0, 0.08);
}

.item:first-child {
  border-top: 0;
}

.title {
  font-weight: 800;
  font-size: 13px;
}

.sub {
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-muted);
}

.sep {
  margin: 0 6px;
}

.actions {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.tute-btn-ghost.danger {
  border-color: rgba(217, 48, 38, 0.3);
  color: #d93026;
}

.tute-btn-ghost.danger:hover {
  background: rgba(217, 48, 38, 0.06);
  color: #d93026;
}

.empty {
  padding: 14px 14px;
  color: var(--text-muted);
  box-shadow: var(--shadow-card);
  border-radius: 4px;
}
</style>
