<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { contentGetCase, userIsFavorite, userSetFavorite, type ApiResponse, type CaseDetail } from '../api/tauri'
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
const detail = ref<ApiResponse<CaseDetail> | null>(null)
const isFavorite = ref(false)
const coverSrc = computed(() => {
  if (!detail.value?.ok) return undefined
  return resolveContentAssetUrl(detail.value.data.coverUrl) ?? undefined
})

const copyBtnEl = ref<HTMLElement | null>(null)
const favBtnEl = ref<HTMLElement | null>(null)

async function load() {
  const cid = id.value.trim()
  if (!cid) return
  loading.value = true
  try {
    detail.value = await contentGetCase({ id: cid })
    isFavorite.value = false
    if (detail.value.ok) {
      const f = await userIsFavorite({ entityType: 'case', entityId: detail.value.data.id })
      if (f.ok) isFavorite.value = f.data.isFavorite
    }
  } finally {
    loading.value = false
  }
}

watch(id, load, { immediate: true })

function back() {
  router.push('/cases')
}

async function toggleFavorite() {
  if (!detail.value?.ok) return
  const next = !isFavorite.value
  const r = await userSetFavorite({ entityType: 'case', entityId: detail.value.data.id, isFavorite: next })
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
  const c = detail.value.data
  const ok = await copyText(
    `${c.title}\n场景：${c.scene}\n\n摘要：${c.summary}\n\n情景描述：\n${c.body}\n\n违纪风险：\n${c.violation}\n\n正确做法：\n${c.correctAction}`,
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
      <div class="headTitle">案例详情</div>
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
        <img class="coverImg" :src="coverSrc" :alt="detail.data.title" />
      </div>
      <MediaPlaceholder v-else kind="image" label="案例封面占位（后续可替换为图片/视频封面）" ratio="21 / 9" />

      <div class="title">{{ detail.data.title }}</div>
      <div class="meta">
        <span class="tute-badge gold">{{ detail.data.scene }}</span>
        <span class="tute-badge">{{ detail.data.id }}</span>
      </div>

      <div class="block">
        <div class="k">摘要</div>
        <div class="v">{{ detail.data.summary }}</div>
      </div>
      <div class="block">
        <div class="k">情景描述</div>
        <div class="v">{{ detail.data.body }}</div>
      </div>
      <div v-if="detail.data.violation" class="block">
        <div class="k">违纪风险</div>
        <div class="v">{{ detail.data.violation }}</div>
      </div>
      <div v-if="detail.data.correctAction" class="block">
        <div class="k">正确做法</div>
        <div class="v">{{ detail.data.correctAction }}</div>
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

.block {
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 10px;
  background: #ffffff;
  padding: 12px 12px;
}

.k {
  font-weight: 900;
  font-size: 12px;
}

.v {
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.75;
  white-space: pre-wrap;
}

.err {
  font-size: 12px;
  color: #d93026;
}
</style>
