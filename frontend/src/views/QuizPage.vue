<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
  quizGetProgress,
  quizStartSession,
  quizSubmitAnswer,
  type ApiResponse,
  type QuizProgress,
  type QuizQuestion,
  type QuizStartSessionResult,
  type QuizSubmitAnswerResult,
} from '../api/tauri'
import { useToastStore } from '../stores/toast'

const toast = useToastStore()
const progress = ref<ApiResponse<QuizProgress> | null>(null)
const session = ref<ApiResponse<QuizStartSessionResult> | null>(null)
const index = ref(0)
const last = ref<ApiResponse<QuizSubmitAnswerResult> | null>(null)
const loading = ref(false)
const mode = ref<'daily' | 'stage' | 'practice'>('daily')
const finished = ref(false)
const answeredCount = ref(0)
const correctCount = ref(0)
const pointsStart = ref(0)
const pointsEnd = ref(0)

const current = computed<QuizQuestion | null>(() => {
  if (!session.value?.ok) return null
  return session.value.data.questions[index.value] ?? null
})

const totalCount = computed(() => (session.value?.ok ? session.value.data.questions.length : 0))
const progressPct = computed(() => {
  if (!totalCount.value) return 0
  return Math.round((Math.min(index.value, totalCount.value) / totalCount.value) * 100)
})

async function refreshProgress() {
  progress.value = await quizGetProgress()
}

async function start(modeArg: 'daily' | 'stage' | 'practice') {
  loading.value = true
  try {
    await refreshProgress()
    pointsStart.value = progress.value?.ok ? progress.value.data.totalPoints : 0
    mode.value = modeArg
    session.value = await quizStartSession({ mode: modeArg })
    index.value = 0
    last.value = null
    finished.value = false
    answeredCount.value = 0
    correctCount.value = 0
    await refreshProgress()
    pointsEnd.value = progress.value?.ok ? progress.value.data.totalPoints : 0
    if (session.value.ok && session.value.data.questions.length === 0) {
      toast.info('暂无题目（请先确保 content.db 中有 questions）')
    }
  } finally {
    loading.value = false
  }
}

async function answer(optKey: string) {
  if (!session.value?.ok) return
  const q = current.value
  if (!q) return
  last.value = await quizSubmitAnswer({
    sessionId: session.value.data.sessionId,
    questionId: q.id,
    answer: optKey,
  })
  await refreshProgress()
  if (last.value.ok) {
    answeredCount.value += 1
    if (last.value.data.isCorrect) correctCount.value += 1
    pointsEnd.value = last.value.data.totalPoints
  } else {
    toast.error(`${last.value.error.code} · ${last.value.error.message}`)
  }
}

function next() {
  last.value = null
  index.value += 1
}

function finishSession() {
  finished.value = true
  last.value = null
}

function resetSession() {
  session.value = null
  last.value = null
  finished.value = false
  index.value = 0
  answeredCount.value = 0
  correctCount.value = 0
}

onMounted(refreshProgress)
</script>

<template>
  <div>
    <div class="toolbar tute-card">
      <div class="actions">
        <button class="tute-btn" type="button" :disabled="loading" @click="() => start('daily')">
          {{ loading ? '启动中…' : '每日挑战' }}
        </button>
        <button class="tute-btn-ghost" type="button" :disabled="loading" @click="() => start('stage')">闯关练习</button>
        <button class="tute-btn-ghost" type="button" :disabled="loading" @click="() => start('practice')">专项练习</button>
      </div>
      <div class="right">
        <span v-if="progress?.ok" class="tute-muted">当前积分：{{ progress.data.totalPoints }}</span>
        <span v-else-if="progress && !progress.ok" class="err">{{ progress.error.code }} · {{ progress.error.message }}</span>
      </div>
    </div>

    <div v-if="session?.ok" class="panel tute-card">
      <div class="panelTop">
        <div class="k">模式</div>
        <div class="v">{{ mode === 'daily' ? '每日挑战' : mode === 'stage' ? '闯关练习' : '专项练习' }}</div>
      </div>
      <div class="panelTop">
        <div class="k">进度</div>
        <div class="v">{{ Math.min(index + 1, session.data.questions.length) }} / {{ session.data.questions.length }}</div>
      </div>
      <div class="bar">
        <div class="barFill" :style="{ width: `${progressPct}%` }"></div>
      </div>

      <div v-if="finished" class="summary">
        <div class="sumTitle">本次结果</div>
        <div class="sumGrid">
          <div class="sumItem">
            <div class="sumK">答题数</div>
            <div class="sumV">{{ answeredCount }} / {{ totalCount }}</div>
          </div>
          <div class="sumItem">
            <div class="sumK">正确数</div>
            <div class="sumV">{{ correctCount }}</div>
          </div>
          <div class="sumItem">
            <div class="sumK">正确率</div>
            <div class="sumV">{{ totalCount ? Math.round((correctCount / totalCount) * 100) : 0 }}%</div>
          </div>
          <div class="sumItem">
            <div class="sumK">得分变化</div>
            <div class="sumV">{{ pointsEnd - pointsStart }}</div>
          </div>
        </div>
        <div class="resActions">
          <button class="tute-btn" type="button" :disabled="loading" @click="() => start(mode)">再来一组</button>
          <button class="tute-btn-ghost" type="button" @click="resetSession">退出</button>
        </div>
      </div>

      <div v-else-if="current" class="q">
        <div class="stem">{{ current.stem }}</div>
        <div class="opts">
          <button
            v-for="o in current.options"
            :key="o.key"
            class="opt"
            type="button"
            :disabled="!!last"
            @click="answer(o.key)"
          >
            <span class="optKey">{{ o.key }}</span>
            <span class="optText">{{ o.text }}</span>
          </button>
        </div>

        <div v-if="last" class="result">
          <div v-if="last.ok" class="resultBody">
            <div class="resTitle" :class="{ ok: last.data.isCorrect, bad: !last.data.isCorrect }">
              {{ last.data.isCorrect ? '回答正确' : '回答错误' }}
            </div>
            <div class="resLine">正确答案：{{ last.data.correctAnswer }}</div>
            <div class="resLine">本题得分：{{ last.data.pointsDelta }} · 总积分：{{ last.data.totalPoints }}</div>
            <div v-if="last.data.explanation" class="resExplain">{{ last.data.explanation }}</div>
            <div class="resActions">
              <button
                v-if="session.data.questions[index + 1]"
                class="tute-btn"
                type="button"
                @click="next"
              >
                下一题
              </button>
              <button v-else class="tute-btn" type="button" @click="finishSession">完成</button>
            </div>
          </div>
          <div v-else class="resultBody">
            <div class="err">{{ last.error.code }} · {{ last.error.message }}</div>
          </div>
        </div>
      </div>
      <div v-else class="empty">暂无题目（请先确保 content.db 中有 questions）</div>
    </div>

    <div v-else class="hint">
      选择一个模式开始答题。离线答题会写入本地；登录后会自动同步积分与答题记录。
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

.actions {
  display: flex;
  gap: 10px;
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

.panel {
  margin-top: 16px;
  padding: 14px 14px;
}

.panelTop {
  display: grid;
  grid-template-columns: 70px 1fr;
  gap: 10px;
  margin-bottom: 10px;
}

.bar {
  height: 8px;
  border-radius: 999px;
  background: rgba(0, 0, 0, 0.08);
  overflow: hidden;
  margin: 8px 0 4px;
}

.barFill {
  height: 100%;
  background: rgba(139, 26, 92, 0.8);
  border-radius: 999px;
}

.k {
  font-size: 12px;
  font-weight: 800;
  color: var(--text-primary);
}

.v {
  font-size: 12px;
  color: var(--text-muted);
}

.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
    monospace;
}

.q {
  margin-top: 12px;
  padding: 12px 12px;
  border-radius: 4px;
  background: #fafafa;
  border: 1px solid rgba(0, 0, 0, 0.08);
}

.summary {
  margin-top: 12px;
  padding: 12px 12px;
  border-radius: 8px;
  background: #fafafa;
  border: 1px solid rgba(0, 0, 0, 0.08);
}

.sumTitle {
  font-weight: 900;
  font-size: 13px;
}

.sumGrid {
  margin-top: 10px;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.sumItem {
  padding: 10px 10px;
  border-radius: 8px;
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.08);
}

.sumK {
  font-size: 12px;
  color: var(--text-muted);
}

.sumV {
  margin-top: 8px;
  font-weight: 900;
  font-size: 14px;
  color: var(--text-primary);
}

.stem {
  font-weight: 900;
  font-size: 13px;
  line-height: 1.6;
}

.opts {
  margin-top: 12px;
  display: grid;
  gap: 10px;
}

.opt {
  text-align: left;
  width: 100%;
  padding: 12px 12px;
  border-radius: 4px;
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.12);
  color: var(--text-primary);
  cursor: pointer;
  display: grid;
  grid-template-columns: 26px 1fr;
  gap: 10px;
  align-items: start;
}

.opt:hover {
  box-shadow: var(--shadow-card);
}

.opt:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

.optKey {
  font-weight: 900;
}

.optText {
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.6;
}

.result {
  margin-top: 12px;
  padding: 12px 12px;
  border-radius: 4px;
  border: 1px solid rgba(0, 0, 0, 0.12);
  background: #ffffff;
}

.resTitle {
  font-weight: 900;
  font-size: 13px;
}

.resTitle.ok {
  color: #188038;
}

.resTitle.bad {
  color: #d93026;
}

.resLine {
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-muted);
}

.resExplain {
  margin-top: 10px;
  font-size: 12px;
  color: var(--text-primary);
  line-height: 1.7;
  white-space: pre-wrap;
}

.resActions {
  margin-top: 12px;
  display: flex;
  gap: 10px;
  justify-content: flex-end;
}

.empty {
  margin-top: 12px;
  padding: 14px;
  border-radius: 4px;
  border: 1px dashed rgba(0, 0, 0, 0.2);
  color: var(--text-muted);
  font-size: 12px;
}

.hint {
  margin-top: 16px;
  font-size: 12px;
  color: var(--text-muted);
}
</style>
