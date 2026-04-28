<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { contentGetRegulation, userIsFavorite, userSetFavorite, type ApiResponse, type RegulationDetail } from '../api/tauri'
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
const regulation = ref<ApiResponse<RegulationDetail> | null>(null)
const isFavorite = ref(false)
const activeSectionId = ref<string>('')
const coverSrc = computed(() => {
  if (!regulation.value?.ok) return undefined
  return resolveContentAssetUrl(regulation.value.data.coverUrl) ?? undefined
})

const copyBtnEl = ref<HTMLElement | null>(null)
const favBtnEl = ref<HTMLElement | null>(null)

async function load() {
  const rid = id.value.trim()
  if (!rid) return
  loading.value = true
  try {
    regulation.value = await contentGetRegulation({ id: rid })
    isFavorite.value = false
    if (regulation.value.ok) {
      const f = await userIsFavorite({ entityType: 'regulation', entityId: regulation.value.data.id })
      if (f.ok) isFavorite.value = f.data.isFavorite
      const first = regulation.value.data.sections[0]?.id ?? ''
      if (first) activeSectionId.value = first
    }
  } finally {
    loading.value = false
  }
}

watch(id, load, { immediate: true })

function back() {
  router.push('/regulations')
}

const activeSection = computed(() => {
  if (!regulation.value?.ok) return null
  const sid = activeSectionId.value
  return regulation.value.data.sections.find((s) => s.id === sid) ?? regulation.value.data.sections[0] ?? null
})

function selectSection(id: string) {
  activeSectionId.value = id
}

const activeIndex = computed(() => {
  if (!regulation.value?.ok) return -1
  const sid = activeSectionId.value
  return regulation.value.data.sections.findIndex((s) => s.id === sid)
})

function prevSection() {
  if (!regulation.value?.ok) return
  const idx = activeIndex.value
  if (idx <= 0) return
  activeSectionId.value = regulation.value.data.sections[idx - 1].id
}

function nextSection() {
  if (!regulation.value?.ok) return
  const idx = activeIndex.value
  if (idx < 0) return
  const next = regulation.value.data.sections[idx + 1]
  if (!next) return
  activeSectionId.value = next.id
}

async function toggleFavorite() {
  if (!regulation.value?.ok) return
  const next = !isFavorite.value
  const r = await userSetFavorite({ entityType: 'regulation', entityId: regulation.value.data.id, isFavorite: next })
  if (r.ok) {
    isFavorite.value = r.data.isFavorite
    if (favBtnEl.value) animatePop(favBtnEl.value)
    toast.success(r.data.isFavorite ? '已收藏' : '已取消收藏')
  } else {
    toast.error(`${r.error.code} · ${r.error.message}`)
  }
}

async function copyCurrent() {
  if (!regulation.value?.ok) return
  const r = regulation.value.data
  const parts: string[] = []
  parts.push(r.title)
  parts.push(`层级：${r.level}`)
  if (r.source) parts.push(`来源：${r.source}`)
  if (r.publishedAt) parts.push(`发布日期：${r.publishedAt}`)
  parts.push('')
  for (const s of r.sections) {
    const head = `${s.chapter ?? ''} ${s.articleNo ?? ''} ${s.title ?? ''}`.trim()
    parts.push(head)
    if (s.body) parts.push(s.body)
    parts.push('')
  }
  const ok = await copyText(parts.join('\n'))
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
    <div v-if="loading" class="tute-muted">加载中…</div>
    <div v-else-if="regulation?.ok" class="detail">
      <div class="topBar">
        <button class="tute-btn-ghost" type="button" @click="back">返回</button>
        <div class="topTitle">法规学习</div>
        <div class="topActions">
          <button ref="copyBtnEl" class="tute-btn-ghost" type="button" @click="copyCurrent">复制全文</button>
          <button ref="favBtnEl" class="tute-btn" type="button" @click="toggleFavorite">
            {{ isFavorite ? '取消收藏' : '收藏' }}
          </button>
        </div>
      </div>

      <div class="hero">
        <div v-if="coverSrc" class="heroCover" :style="{ aspectRatio: '21 / 9' }">
          <img class="heroCoverImg" :src="coverSrc" :alt="regulation.data.title" />
        </div>
        <MediaPlaceholder v-else kind="image" label="封面图占位（法规学习）" ratio="21 / 9" />
        <div class="heroInfo">
          <div class="heroTitle">{{ regulation.data.title }}</div>
          <div class="heroMeta">
            <span class="tute-badge gold">{{ regulation.data.level }}</span>
            <span class="tute-badge">{{ regulation.data.id }}</span>
            <span v-if="regulation.data.publishedAt" class="tute-badge">{{ regulation.data.publishedAt }}</span>
            <span v-if="regulation.data.source" class="tute-badge">{{ regulation.data.source }}</span>
          </div>
          <div class="progress">
            <div class="progressBar"><div class="progressFill"></div></div>
            <div class="progressText">学习进度占位</div>
          </div>
        </div>
      </div>

      <div class="grid">
        <div class="main">
          <div class="panel">
            <div class="panelTitle">学习资源</div>
            <div class="mediaGrid">
              <MediaPlaceholder kind="video" label="视频占位（后续可替换为讲解/导学）" ratio="16 / 9" />
              <MediaPlaceholder kind="image" label="配图占位" ratio="16 / 9" />
              <MediaPlaceholder kind="image" label="配图占位" ratio="16 / 9" />
            </div>
          </div>

          <div class="panel">
            <div class="panelTitle">条款内容</div>
            <div v-if="regulation.data.sections.length > 0 && activeSection" class="sec">
              <div class="secTitle">{{ activeSection.chapter ?? '' }} {{ activeSection.articleNo ?? '' }} {{ activeSection.title ?? '' }}</div>
              <div class="secBody">{{ activeSection.body }}</div>
              <div class="secNav">
                <button class="tute-btn-ghost" type="button" :disabled="activeIndex <= 0" @click="prevSection">上一条</button>
                <div class="secPos tute-muted">{{ activeIndex + 1 }} / {{ regulation.data.sections.length }}</div>
                <button
                  class="tute-btn-ghost"
                  type="button"
                  :disabled="activeIndex < 0 || activeIndex >= regulation.data.sections.length - 1"
                  @click="nextSection"
                >
                  下一条
                </button>
              </div>
            </div>
            <div v-else class="tute-muted">暂无条款</div>
          </div>
        </div>

        <aside class="aside">
          <div v-if="regulation.data.sections.length > 0" class="toc">
            <div class="tocTitle">目录</div>
            <div class="tocList">
              <button
                v-for="s in regulation.data.sections"
                :key="s.id"
                class="tocBtn"
                type="button"
                :class="{ active: s.id === activeSectionId }"
                @click="selectSection(s.id)"
              >
                {{ s.articleNo ?? '' }} {{ s.title ?? '' }}
              </button>
            </div>
          </div>
          <div class="asideTip">
            <div class="asideTitle">提示</div>
            <div class="asideBody">后续可在“学习资源”中接入视频/图片，并记录学习进度。</div>
          </div>
        </aside>
      </div>
    </div>
    <div v-else-if="regulation && !regulation.ok" class="err">{{ regulation.error.code }} · {{ regulation.error.message }}</div>
    <div v-else class="tute-muted">暂无数据</div>
  </div>
</template>

<style scoped>
.wrap {
  display: grid;
  gap: 12px;
}

.detail {
  display: grid;
  gap: 12px;
}

.topBar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  flex-wrap: wrap;
}

.topTitle {
  font-weight: 900;
  font-size: 14px;
}

.topActions {
  display: flex;
  gap: 10px;
  align-items: center;
}

.hero {
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 14px;
  overflow: hidden;
  background: #ffffff;
}

.heroCover {
  background: #ffffff;
}

.heroCoverImg {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.heroInfo {
  padding: 12px 12px;
  display: grid;
  gap: 10px;
}

.heroTitle {
  font-weight: 900;
  font-size: 16px;
  line-height: 1.3;
}

.heroMeta {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.progress {
  display: grid;
  gap: 8px;
}

.progressBar {
  height: 10px;
  border-radius: 999px;
  background: rgba(0, 0, 0, 0.08);
  overflow: hidden;
}

.progressFill {
  height: 100%;
  width: 22%;
  background: linear-gradient(90deg, rgba(139, 26, 92, 0.9), rgba(168, 61, 122, 0.9));
}

.progressText {
  font-size: 12px;
  color: var(--text-muted);
}

.grid {
  display: grid;
  grid-template-columns: 1fr 320px;
  gap: 12px;
  align-items: start;
}

.main {
  min-width: 0;
  display: grid;
  gap: 12px;
}

.panel {
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 12px;
  background: #ffffff;
  padding: 12px 12px;
}

.panelTitle {
  font-weight: 900;
  font-size: 13px;
}

.mediaGrid {
  margin-top: 10px;
  display: grid;
  gap: 10px;
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.aside {
  position: sticky;
  top: 12px;
  display: grid;
  gap: 12px;
}

.toc {
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 12px;
  padding: 12px 12px;
  background: #ffffff;
}

.tocTitle {
  font-weight: 900;
  font-size: 13px;
}

.tocList {
  margin-top: 10px;
  display: grid;
  gap: 8px;
}

.tocBtn {
  border: 1px solid rgba(0, 0, 0, 0.12);
  background: #ffffff;
  border-radius: 8px;
  padding: 8px 10px;
  cursor: pointer;
  font-size: 12px;
  text-align: left;
}

.tocBtn:hover {
  border-color: rgba(139, 26, 92, 0.25);
}

.sec {
  margin-top: 10px;
  padding: 12px 12px;
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 10px;
  background: #ffffff;
}

.secTitle {
  font-weight: 900;
  font-size: 13px;
}

.secBody {
  margin-top: 10px;
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.75;
  white-space: pre-wrap;
}

.secNav {
  margin-top: 12px;
  display: flex;
  justify-content: space-between;
  gap: 10px;
  align-items: center;
}

.secPos {
  font-size: 12px;
}

.tocBtn.active {
  border-color: rgba(139, 26, 92, 0.35);
  box-shadow: 0 0 0 3px rgba(139, 26, 92, 0.12);
}

.err {
  font-size: 12px;
  color: #d93026;
}

.asideTip {
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 12px;
  padding: 12px 12px;
  background: rgba(139, 26, 92, 0.03);
}

.asideTitle {
  font-weight: 900;
  font-size: 13px;
}

.asideBody {
  margin-top: 10px;
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.6;
}

@media (max-width: 980px) {
  .grid {
    grid-template-columns: 1fr;
  }
  .aside {
    position: static;
  }
  .mediaGrid {
    grid-template-columns: 1fr;
  }
}
</style>
