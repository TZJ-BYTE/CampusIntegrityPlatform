<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useSyncStore } from '../stores/sync'
import { useUiStore } from '../stores/ui'
import { animateIn, animateOut } from '../utils/motion'

const sync = useSyncStore()
const ui = useUiStore()

const open = computed(() => ui.overlay === 'onboarding')
const shown = ref(false)
const cardEl = ref<HTMLElement | null>(null)

const isLoggedIn = computed(() => !!(sync.auth?.ok && sync.auth.data.isLoggedIn))
const username = computed(() => (sync.auth?.ok ? (sync.auth.data.username ?? '').trim() : ''))

function markOnboarded() {
  try {
    localStorage.setItem('cip:onboarded', '1')
  } catch {}
}

async function close() {
  if (cardEl.value) await animateOut(cardEl.value, { y: 10, scale: 0.99, opacity: 0 })
  ui.close()
}

async function goAccount() {
  markOnboarded()
  await close()
  ui.open('auth')
}

async function goOffline() {
  markOnboarded()
  await close()
}

watch(
  () => open.value,
  async (v) => {
    if (v) {
      shown.value = true
      await nextTick()
      if (cardEl.value) animateIn(cardEl.value, { from: { y: 14, scale: 0.98, opacity: 0 }, to: { y: 0, scale: 1, opacity: 1, duration: 0.3 } })
      return
    }
    shown.value = false
  },
  { immediate: true },
)
</script>

<template>
  <div v-if="shown" class="mask" role="dialog" aria-modal="true">
    <div ref="cardEl" class="card">
      <div class="title">欢迎使用</div>
      <div class="sub">校园廉洁教育平台 · 离线优先</div>

      <div class="hint">离线操作始终写入本地；联网后会自动同步，不需要手动确认。</div>

      <div v-if="isLoggedIn" class="tip">
        当前已登录<span v-if="username">：账号 {{ username }}</span>
      </div>

      <div class="actions">
        <button v-if="!isLoggedIn" class="tute-btn" type="button" @click="goAccount">登录并开启云同步</button>
        <button class="tute-btn-ghost" type="button" @click="goOffline">
          {{ isLoggedIn ? '进入主页' : '先离线使用' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  display: grid;
  place-items: center;
  padding: 16px;
  z-index: 999;
}

.card {
  width: min(520px, 100%);
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 10px;
  background: #ffffff;
  padding: 16px 16px;
  box-shadow: 0 18px 60px rgba(0, 0, 0, 0.22);
}

.title {
  font-weight: 900;
  font-size: 18px;
  color: var(--text-primary);
}

.sub {
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-muted);
}

.hint {
  margin-top: 10px;
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.6;
}

.tip {
  margin-top: 12px;
  padding: 10px 12px;
  border-radius: 6px;
  background: #fafafa;
  border: 1px solid rgba(0, 0, 0, 0.06);
  font-size: 12px;
  color: var(--text-secondary);
}

.actions {
  margin-top: 14px;
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  justify-content: flex-end;
}
</style>
