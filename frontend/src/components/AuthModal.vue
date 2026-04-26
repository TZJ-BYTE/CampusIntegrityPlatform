<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useSyncStore } from '../stores/sync'
import { useUiStore } from '../stores/ui'

const sync = useSyncStore()
const ui = useUiStore()

const open = computed(() => ui.overlay === 'auth')
const username = ref('')
const password = ref('')
const message = ref('')

watch(
  open,
  async (v) => {
    if (!v) return
    message.value = ''
    username.value = ''
    password.value = ''
    await sync.refresh()
  },
  { immediate: false },
)

function close() {
  ui.close()
}

async function doLogin() {
  await sync.login(username.value.trim() || undefined, password.value || undefined)
  if (sync.message) message.value = sync.message
  if (sync.auth?.ok && sync.auth.data.isLoggedIn) ui.close()
}
</script>

<template>
  <div v-if="open" class="mask" role="dialog" aria-modal="true">
    <div class="card">
      <div class="head">
        <div class="title">登录</div>
        <button class="x" type="button" @click="close">×</button>
      </div>

      <div class="hint">登录后会保持会话，自动同步无需手动确认。</div>

      <div class="row">
        <div class="k">用户名</div>
        <input v-model="username" class="tute-input input" placeholder="请输入用户名" />
      </div>

      <div class="row">
        <div class="k">密码</div>
        <input v-model="password" class="tute-input input" type="password" placeholder="请输入密码" />
      </div>

      <div v-if="message" class="message">{{ message }}</div>

      <div class="actions">
        <button class="tute-btn-ghost" type="button" :disabled="sync.busy" @click="close">取消</button>
        <button class="tute-btn" type="button" :disabled="sync.busy" @click="doLogin">{{ sync.busy ? '登录中…' : '登录' }}</button>
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
  padding: 14px 14px;
  box-shadow: 0 18px 60px rgba(0, 0, 0, 0.22);
}

.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.title {
  font-weight: 900;
  font-size: 14px;
}

.x {
  border: 0;
  background: transparent;
  cursor: pointer;
  font-size: 18px;
  line-height: 1;
  padding: 4px 6px;
  color: var(--text-muted);
}

.hint {
  margin-top: 10px;
  font-size: 12px;
  color: var(--text-muted);
}

.row {
  margin-top: 10px;
  padding: 12px 12px;
  border-radius: 4px;
  background: #fafafa;
  border: 1px solid rgba(0, 0, 0, 0.06);
}

.k {
  font-size: 12px;
  font-weight: 700;
  color: var(--text-secondary);
}

.input {
  margin-top: 8px;
  width: 100%;
}

.message {
  margin-top: 10px;
  padding: 10px 12px;
  border-radius: 6px;
  background: rgba(139, 26, 92, 0.06);
  border: 1px solid rgba(139, 26, 92, 0.18);
  color: var(--text-secondary);
  font-size: 12px;
}

.actions {
  margin-top: 12px;
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  justify-content: flex-end;
}
</style>
