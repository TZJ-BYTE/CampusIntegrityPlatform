<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { contentGetVenue, userIsFavorite, userSetFavorite, type ApiResponse, type VenueDetail } from '../api/tauri'
import MediaPlaceholder from '../components/MediaPlaceholder.vue'
import { useToastStore } from '../stores/toast'
import { copyText } from '../utils/copy'
import { resolveContentAssetUrl } from '../utils/contentAsset'
import { animatePop } from '../utils/motion'

const route = useRoute()
const router = useRouter()
const toast = useToastStore()

const id = computed(() => (route.params.id as string | undefined) ?? '')
const loading = ref(false)
const detail = ref<ApiResponse<VenueDetail> | null>(null)
const isFavorite = ref(false)
const coverSrc = computed(() => {
  if (!detail.value?.ok) return undefined
  return resolveContentAssetUrl(detail.value.data.coverUrl) ?? undefined
})

const copyBtnEl = ref<HTMLElement | null>(null)
const favBtnEl = ref<HTMLElement | null>(null)

async function load() {
  const vid = id.value.trim()
  if (!vid) return
  loading.value = true
  try {
    detail.value = await contentGetVenue({ id: vid })
    isFavorite.value = false
    if (detail.value.ok) {
      const f = await userIsFavorite({ entityType: 'venue', entityId: detail.value.data.id })
      if (f.ok) isFavorite.value = f.data.isFavorite
    }
  } finally {
    loading.value = false
  }
}

watch(id, load, { immediate: true })

function back() {
  router.push('/venues')
}

async function toggleFavorite() {
  if (!detail.value?.ok) return
  const next = !isFavorite.value
  const r = await userSetFavorite({ entityType: 'venue', entityId: detail.value.data.id, isFavorite: next })
  if (r.ok) {
    isFavorite.value = r.data.isFavorite
    if (favBtnEl.value) animatePop(favBtnEl.value)
    toast.success(r.data.isFavorite ? '已收藏' : '已取消收藏')
  } else {
    toast.error(`${r.error.code} · ${r.error.message}`)
  }
}

async function copyCurrent() {
  if (!detail.value?.ok) return
  const v = detail.value.data
  const ok = await copyText(
    `${v.name}\n类型：${v.type}\n地点：${v.location ?? '—'}\n开放时间：${v.openHours ?? '—'}\n联系方式：${v.contact ?? '—'}\n\n${v.description ?? ''}`,
  )
  if (ok) {
    if (copyBtnEl.value) animatePop(copyBtnEl.value)
    toast.success('已复制')
  } else {
    toast.error('复制失败')
  }
}
</script>

<template>
  <div class="wrap">
    <div class="head">
      <button class="tute-btn-ghost" type="button" @click="back">返回</button>
      <div class="headTitle">场所详情</div>
      <div class="headActions">
        <button ref="copyBtnEl" class="tute-btn-ghost" type="button" :disabled="!detail?.ok" @click="copyCurrent">复制</button>
        <button ref="favBtnEl" class="tute-btn" type="button" :disabled="!detail?.ok" @click="toggleFavorite">
          {{ isFavorite ? '取消收藏' : '收藏' }}
        </button>
      </div>
    </div>

    <div v-if="loading" class="tute-muted">加载中…</div>
    <div v-else-if="detail?.ok" class="detail">
      <div v-if="coverSrc" class="cover" :style="{ aspectRatio: '21 / 9' }">
        <img class="coverImg" :src="coverSrc" :alt="detail.data.name" />
      </div>
      <MediaPlaceholder v-else kind="image" label="场所封面占位（后续可替换为照片/视频封面）" ratio="21 / 9" />

      <div class="title">{{ detail.data.name }}</div>
      <div class="meta">
        <span class="tute-badge gold">{{ detail.data.type }}</span>
        <span class="tute-badge">{{ detail.data.id }}</span>
      </div>

      <div class="kv">
        <div class="row"><div class="k">地点</div><div class="v">{{ detail.data.location ?? '—' }}</div></div>
        <div class="row"><div class="k">开放时间</div><div class="v">{{ detail.data.openHours ?? '—' }}</div></div>
        <div class="row"><div class="k">联系方式</div><div class="v">{{ detail.data.contact ?? '—' }}</div></div>
      </div>

      <div class="mediaPanel">
        <div class="k">参观资源</div>
        <div class="mediaGrid">
          <MediaPlaceholder kind="video" label="导览视频占位" ratio="16 / 9" />
          <MediaPlaceholder kind="image" label="场所照片占位" ratio="16 / 9" />
          <MediaPlaceholder kind="image" label="场所照片占位" ratio="16 / 9" />
        </div>
      </div>

      <div v-if="detail.data.description" class="desc">
        <div class="k">简介</div>
        <div class="v">{{ detail.data.description }}</div>
      </div>
    </div>
    <div v-else-if="detail && !detail.ok" class="err">{{ detail.error.code }} · {{ detail.error.message }}</div>
    <div v-else class="tute-muted">暂无数据</div>
  </div>
</template>

<style scoped>
.wrap {
  display: grid;
  gap: 12px;
}

.cover {
  border-radius: 12px;
  overflow: hidden;
  border: 1px solid rgba(0, 0, 0, 0.08);
  background: #ffffff;
}

.coverImg {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  flex-wrap: wrap;
}

.headTitle {
  font-weight: 900;
  font-size: 14px;
}

.headActions {
  display: flex;
  gap: 10px;
  align-items: center;
}

.detail {
  display: grid;
  gap: 12px;
}

.title {
  font-weight: 900;
  font-size: 16px;
  line-height: 1.3;
}

.meta {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.kv {
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 10px;
  background: #ffffff;
  padding: 12px 12px;
  display: grid;
  gap: 10px;
}

.row {
  display: grid;
  grid-template-columns: 84px 1fr;
  gap: 10px;
  align-items: start;
}

.k {
  font-weight: 900;
  font-size: 12px;
  color: var(--text-primary);
}

.v {
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.7;
  white-space: pre-wrap;
}

.desc {
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 10px;
  background: #ffffff;
  padding: 12px 12px;
  display: grid;
  gap: 8px;
}

.mediaPanel {
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 10px;
  background: #ffffff;
  padding: 12px 12px;
  display: grid;
  gap: 10px;
}

.mediaGrid {
  display: grid;
  gap: 10px;
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.err {
  font-size: 12px;
  color: #d93026;
}

@media (max-width: 980px) {
  .mediaGrid {
    grid-template-columns: 1fr;
  }
}
</style>
