<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useSyncStore } from '../stores/sync'
import { useUiStore } from '../stores/ui'

const sync = useSyncStore()
const ui = useUiStore()

const open = computed(() => ui.overlay === 'account')
const message = ref('')

function fmtTime(ms: number) {
  if (!ms) return '—'
  try {
    return new Date(ms).toLocaleString()
  } catch {
    return '—'
  }
}

const isLoggedIn = computed(() => !!(sync.auth?.ok && sync.auth.data.isLoggedIn))
const username = computed(() => (sync.auth?.ok ? (sync.auth.data.username ?? '').trim() : ''))
const baseUrl = computed(() => (sync.auth?.ok ? (sync.auth.data.baseUrl ?? '').trim() : ''))
const pending = computed(() => (sync.sync?.ok ? sync.sync.data.pendingCount : 0))

const cloudStatus = computed(() => {
  if (!sync.auth?.ok) return '云 —'
  if (!sync.auth.data.isLoggedIn) return '云 未连接'
  if (sync.busy) return '云 同步中'
  if (sync.lastErrorAt && (!sync.lastOkAt || sync.lastErrorAt > sync.lastOkAt)) return '云 异常'
  return '云 已连接'
})

watch(
  open,
  async (v) => {
    if (!v) return
    message.value = ''
    await sync.refresh()
  },
  { immediate: false },
)

function close() {
  ui.close()
}

function openAuth() {
  ui.open('auth')
}

function openDiagnostics() {
  ui.open('diagnostics')
}

async function doLogout() {
  await sync.logout()
  if (sync.message) message.value = sync.message
  await sync.refresh()
}

async function doSwitchAccount() {
  await sync.logout()
  await sync.refresh()
  ui.open('auth')
}

async function saveServer() {
  await sync.setServer(sync.serverUrl)
  if (sync.message) message.value = sync.message
}
</script>

<template>
  <div v-if="open" class="mask" role="dialog" aria-modal="true">
    <div class="panel">
      <div class="head">
        <div>
          <div class="title">账号与云同步</div>
          <div class="sub">{{ cloudStatus }}</div>
        </div>
        <button class="x" type="button" @click="close">×</button>
      </div>

      <div class="row">
        <div class="k">状态</div>
        <div class="v">
          <span v-if="isLoggedIn">已连接</span><span v-else>未连接</span>
          <span v-if="baseUrl" class="sep">·</span>
          <span v-if="baseUrl" class="mono">{{ baseUrl }}</span>
          <span v-if="isLoggedIn && username" class="sep">·</span>
          <span v-if="isLoggedIn && username">账号 {{ username }}</span>
        </div>
      </div>

      <div class="row">
        <div class="k">同步</div>
        <div class="tip">
          待同步：{{ pending }} 条
          <span v-if="sync.sync?.ok && sync.sync.data.lastSyncAt"> · 上次同步：{{ new Date(sync.sync.data.lastSyncAt).toLocaleString() }}</span>
          <span v-if="sync.busy"> · 同步中…</span>
        </div>
      </div>

      <div class="actions">
        <button v-if="!isLoggedIn" class="tute-btn" type="button" :disabled="sync.busy" @click="openAuth">登录</button>
        <button v-if="isLoggedIn" class="tute-btn-ghost" type="button" :disabled="sync.busy" @click="doSwitchAccount">
          切换账号
        </button>
        <button v-if="isLoggedIn" class="tute-btn-ghost" type="button" :disabled="sync.busy" @click="doLogout">退出登录</button>
        <button class="tute-btn-ghost" type="button" @click="openDiagnostics">诊断</button>
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
            <button class="tute-btn-ghost" type="button" :disabled="sync.busy" @click="saveServer">保存</button>
          </div>
        </div>
      </div>

      <div class="row">
        <div class="k">诊断摘要</div>
        <div class="tip">
          <div>最近成功：{{ fmtTime(sync.lastOkAt) }}</div>
          <div>最近失败：{{ fmtTime(sync.lastErrorAt) }}</div>
          <div v-if="sync.lastError">错误：{{ sync.lastError }}</div>
        </div>
      </div>

      <div v-if="message" class="message">{{ message }}</div>
    </div>
  </div>
</template>

<style scoped>
.mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.28);
  z-index: 999;
}

.panel {
  position: absolute;
  top: 0;
  right: 0;
  height: 100%;
  width: min(420px, 92vw);
  background: #ffffff;
  border-left: 1px solid rgba(0, 0, 0, 0.08);
  box-shadow: -18px 0 60px rgba(0, 0, 0, 0.18);
  padding: 14px 14px;
  overflow: auto;
}

.head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
}

.title {
  font-weight: 900;
  font-size: 14px;
}

.sub {
  margin-top: 6px;
  font-size: 12px;
  color: var(--text-muted);
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

.row {
  margin-top: 10px;
  padding: 12px 12px;
  border-radius: 8px;
  background: #fafafa;
  border: 1px solid rgba(0, 0, 0, 0.06);
}

.k {
  font-size: 12px;
  font-weight: 700;
  color: var(--text-secondary);
}

.v {
  margin-top: 6px;
  font-size: 12px;
  color: var(--text-primary);
}

.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
  word-break: break-all;
}

.sep {
  margin: 0 6px;
}

.tip {
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.6;
}

.input {
  margin-top: 8px;
  width: 100%;
}

.actions {
  margin-top: 12px;
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

.message {
  margin-top: 10px;
  padding: 10px 12px;
  border-radius: 8px;
  background: rgba(139, 26, 92, 0.06);
  border: 1px solid rgba(139, 26, 92, 0.18);
  color: var(--text-secondary);
  font-size: 12px;
}
</style>
