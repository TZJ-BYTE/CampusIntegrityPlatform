<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { contentGetStory, userIsFavorite, userSetFavorite, type ApiResponse, type StoryDetail } from '../api/tauri'
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
const story = ref<ApiResponse<StoryDetail> | null>(null)
const isFavorite = ref(false)
const coverSrc = computed(() => {
  if (!story.value?.ok) return undefined
  return resolveContentAssetUrl(story.value.data.coverUrl) ?? undefined
})

const favBtnEl = ref<HTMLElement | null>(null)
const copyBtnEl = ref<HTMLElement | null>(null)

async function load() {
  const sid = id.value.trim()
  if (!sid) return
  loading.value = true
  try {
    story.value = await contentGetStory({ id: sid })
    isFavorite.value = false
    if (story.value.ok) {
      const f = await userIsFavorite({ entityType: 'story', entityId: story.value.data.id })
      if (f.ok) isFavorite.value = f.data.isFavorite
    }
  } finally {
    loading.value = false
  }
}

watch(id, load, { immediate: true })

function back() {
  router.push('/stories')
}

async function toggleFavorite() {
  if (!story.value?.ok) return
  const next = !isFavorite.value
  const r = await userSetFavorite({ entityType: 'story', entityId: story.value.data.id, isFavorite: next })
  if (r.ok) {
    isFavorite.value = r.data.isFavorite
    if (favBtnEl.value) animatePop(favBtnEl.value)
    toast.success(r.data.isFavorite ? '已收藏' : '已取消收藏')
  } else {
    toast.error(`${r.error.code} · ${r.error.message}`)
  }
}

async function copyCurrent() {
  if (!story.value?.ok) return
  const s = story.value.data
  const ok = await copyText(`${s.title}\n\n${s.body}\n\n来源：${s.source ?? '—'}`)
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
      <div class="headTitle">故事详情</div>
      <div class="headActions">
        <button ref="copyBtnEl" class="tute-btn-ghost" type="button" :disabled="!story?.ok" @click="copyCurrent">复制</button>
        <button ref="favBtnEl" class="tute-btn" type="button" :disabled="!story?.ok" @click="toggleFavorite">
          {{ isFavorite ? '取消收藏' : '收藏' }}
        </button>
      </div>
    </div>

    <div v-if="loading" class="tute-muted">加载中…</div>
    <div v-else-if="story?.ok" class="detail">
      <div v-if="coverSrc" class="cover" :style="{ aspectRatio: '21 / 9' }">
        <img class="coverImg" :src="coverSrc" :alt="story.data.title" />
      </div>
      <MediaPlaceholder v-else kind="image" label="故事封面占位（后续可替换为图片/视频封面）" ratio="21 / 9" />

      <div class="title">{{ story.data.title }}</div>
      <div class="meta">
        <span class="tute-badge gold">{{ story.data.source ?? '—' }}</span>
        <span class="tute-badge">{{ story.data.id }}</span>
        <span v-if="story.data.dayOfYear" class="tute-badge">第 {{ story.data.dayOfYear }} 天</span>
      </div>
      <div class="body">{{ story.data.body }}</div>
    </div>
    <div v-else-if="story && !story.ok" class="err">{{ story.error.code }} · {{ story.error.message }}</div>
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

.body {
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
