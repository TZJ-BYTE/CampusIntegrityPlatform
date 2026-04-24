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

const progress = ref<ApiResponse<QuizProgress> | null>(null)
const session = ref<ApiResponse<QuizStartSessionResult> | null>(null)
const index = ref(0)
const last = ref<ApiResponse<QuizSubmitAnswerResult> | null>(null)
const loading = ref(false)

const current = computed<QuizQuestion | null>(() => {
  if (!session.value?.ok) return null
  return session.value.data.questions[index.value] ?? null
})

async function refreshProgress() {
  progress.value = await quizGetProgress()
}

async function startDaily() {
  loading.value = true
  try {
    session.value = await quizStartSession({ mode: 'daily' })
    index.value = 0
    last.value = null
    await refreshProgress()
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
}

function next() {
  last.value = null
  index.value += 1
}

onMounted(refreshProgress)
</script>

<template>
  <div>
    <div class="toolbar tute-card">
      <div class="actions">
        <button class="tute-btn" type="button" :disabled="loading" @click="startDaily">
          {{ loading ? '启动中…' : '开始每日挑战' }}
        </button>
        <button class="tute-btn-ghost" type="button" @click="refreshProgress">刷新积分</button>
      </div>
      <div class="right">
        <span v-if="progress?.ok" class="tute-muted">当前积分：{{ progress.data.totalPoints }}</span>
        <span v-else-if="progress && !progress.ok" class="err">{{ progress.error.code }} · {{ progress.error.message }}</span>
      </div>
    </div>

    <div v-if="session?.ok" class="panel tute-card">
      <div class="panelTop">
        <div class="k">会话</div>
        <div class="v mono">{{ session.data.sessionId }}</div>
      </div>
      <div class="panelTop">
        <div class="k">进度</div>
        <div class="v">{{ index + 1 }} / {{ session.data.questions.length }}</div>
      </div>

      <div v-if="current" class="q">
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
              <button v-else class="tute-btn-ghost" type="button" @click="session = null">完成</button>
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
      初版已实现最小闭环：开始会话 → 作答 → 积分累计（本地保存）。后续将补充题库分类、解析页、闯关与徽章。
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
