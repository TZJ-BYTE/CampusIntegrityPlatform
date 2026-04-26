<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useSyncStore } from '../stores/sync'

const sync = useSyncStore()

const message = ref('')
const syncUsername = ref('')
const syncPassword = ref('')

const isLoggedIn = computed(() => !!(sync.auth?.ok && sync.auth.data.isLoggedIn))
const baseUrl = computed(() => (sync.auth?.ok ? (sync.auth.data.baseUrl ?? '').trim() : ''))
const username = computed(() => (sync.auth?.ok ? (sync.auth.data.username ?? '').trim() : ''))

function fmtTime(ms: number) {
  if (!ms) return '—'
  try {
    return new Date(ms).toLocaleString()
  } catch {
    return '—'
  }
}

function setMessage(s: string) {
  message.value = s
}

async function doLogin() {
  await sync.login(syncUsername.value.trim() || undefined, syncPassword.value || undefined)
  if (sync.message) setMessage(sync.message)
}

async function doLogout() {
  await sync.logout()
  if (sync.message) setMessage(sync.message)
}

async function doSwitchAccount() {
  await sync.logout()
  syncUsername.value = ''
  syncPassword.value = ''
  if (sync.message) setMessage(sync.message)
}

onMounted(async () => {
  await sync.refresh()
})
</script>

<template>
  <div class="wrap">
    <div class="section">
      <div class="sectionHead">
        <div class="sectionTitle">账号与云同步</div>
        <div class="actions">
          <button
            v-if="isLoggedIn"
            class="tute-btn-ghost"
            type="button"
            :disabled="sync.busy"
            @click="doSwitchAccount"
          >
            切换账号
          </button>
          <button v-if="isLoggedIn" class="tute-btn-ghost" type="button" :disabled="sync.busy" @click="doLogout">
            退出
          </button>
          <button v-if="!isLoggedIn" class="tute-btn" type="button" :disabled="sync.busy" @click="doLogin">
            登录
          </button>
        </div>
      </div>

      <div class="hint">离线操作只写本地；登录后自动同步（云存储体验）。</div>

      <div class="row">
        <div class="k">连接</div>
        <div class="v">
          <span v-if="sync.auth?.ok">
            <span v-if="isLoggedIn">已连接</span><span v-else>未连接</span>
            <span v-if="baseUrl" class="sep">·</span>
            <span v-if="baseUrl" class="mono">{{ baseUrl }}</span>
            <span v-if="isLoggedIn && username" class="sep">·</span>
            <span v-if="isLoggedIn && username">账号 {{ username }}</span>
          </span>
          <span v-else-if="sync.auth && !sync.auth.ok" class="err">{{ sync.auth.error.code }} · {{ sync.auth.error.message }}</span>
          <span v-else class="tute-muted">加载中…</span>
        </div>
      </div>

      <div v-if="!isLoggedIn" class="row">
        <div class="k">登录</div>
        <input v-model="syncUsername" class="tute-input input" placeholder="用户名" />
        <input v-model="syncPassword" class="tute-input input" type="password" placeholder="密码" style="margin-top: 8px" />
        <div class="hint" style="margin-top: 8px">登录后会保持会话，无需反复输入。</div>
      </div>

      <div class="row">
        <div class="k">同步信息</div>
        <div class="tip" v-if="sync.sync?.ok">
          待同步：{{ sync.sync.data.pendingCount }} 条
          <span v-if="sync.sync.data.lastSyncAt"> · 上次同步：{{ new Date(sync.sync.data.lastSyncAt).toLocaleString() }}</span>
          <span v-if="sync.busy"> · 同步中…</span>
        </div>
        <div class="tip" v-else-if="sync.sync && !sync.sync.ok">状态读取失败：{{ sync.sync.error.code }} · {{ sync.sync.error.message }}</div>
      </div>

      <div class="row">
        <div class="k">高级设置</div>
        <div class="actionsInline">
          <button class="tute-btn-ghost" type="button" :disabled="sync.busy" @click="sync.showAdvanced = !sync.showAdvanced">
            {{ sync.showAdvanced ? '收起' : '展开' }}
          </button>
        </div>
        <div v-if="sync.showAdvanced" class="tip">
          <div class="k" style="margin-top: 10px">服务器地址</div>
          <input v-model="sync.serverUrl" class="tute-input input" placeholder="留空则使用默认地址" />
          <div class="actionsInline">
            <button class="tute-btn-ghost" type="button" :disabled="sync.busy" @click="sync.setServer(sync.serverUrl)">保存</button>
          </div>
        </div>
      </div>

      <div class="row">
        <div class="k">诊断信息</div>
        <div class="tip">
          <div>最近成功：{{ fmtTime(sync.lastOkAt) }}</div>
          <div>最近失败：{{ fmtTime(sync.lastErrorAt) }}</div>
          <div v-if="sync.lastError">错误：{{ sync.lastError }}</div>
          <div v-if="sync.lastRun?.ok">最近一次：push {{ sync.lastRun.data.pushed }} / pull {{ sync.lastRun.data.pulled }}</div>
          <div v-else-if="sync.lastRun && !sync.lastRun.ok">最近一次：{{ sync.lastRun.error.code }} · {{ sync.lastRun.error.message }}</div>
        </div>
      </div>
    </div>

    <div v-if="message" class="message">{{ message }}</div>
  </div>
</template>

<style scoped>
.wrap {
  display: grid;
  gap: 16px;
}

.section {
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 6px;
  background: #ffffff;
  padding: 12px 12px;
}

.sectionHead {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.sectionTitle {
  font-weight: 900;
  font-size: 13px;
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

.tip {
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.6;
}

.actions {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.actionsInline {
  margin-top: 10px;
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.v {
  margin-top: 6px;
  font-size: 12px;
  color: var(--text-primary);
}

.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
    monospace;
  word-break: break-all;
}

.err {
  margin-top: 10px;
  font-size: 12px;
  color: #d93026;
}

.sep {
  margin: 0 6px;
}

.message {
  padding: 10px 12px;
  border-radius: 6px;
  background: rgba(139, 26, 92, 0.06);
  border: 1px solid rgba(139, 26, 92, 0.18);
  color: var(--text-secondary);
  font-size: 12px;
}
</style>
