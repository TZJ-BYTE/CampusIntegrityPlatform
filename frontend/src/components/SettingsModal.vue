<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useAppStore } from '../stores/app'
import { useUiStore } from '../stores/ui'
import {
  appBackupUserDb,
  appDeleteBackup,
  appGetDataDir,
  appListBackups,
  appRestoreUserDb,
  type ApiResponse,
  type BackupsList,
  type DataDirInfo,
} from '../api/tauri'

const ui = useUiStore()
const app = useAppStore()

const open = computed(() => ui.overlay === 'settings')
const dataDir = ref<ApiResponse<DataDirInfo> | null>(null)
const backups = ref<ApiResponse<BackupsList> | null>(null)
const busy = ref(false)
const message = ref('')

function close() {
  ui.close()
}

function setMessage(s: string) {
  message.value = s
}

function formatBytes(n: number) {
  if (!Number.isFinite(n)) return '—'
  if (n < 1024) return `${n} B`
  const kb = n / 1024
  if (kb < 1024) return `${kb.toFixed(1)} KB`
  const mb = kb / 1024
  if (mb < 1024) return `${mb.toFixed(1)} MB`
  const gb = mb / 1024
  return `${gb.toFixed(2)} GB`
}

async function refreshBackups() {
  backups.value = await appListBackups()
}

async function createBackup() {
  busy.value = true
  try {
    const r = await appBackupUserDb()
    if (r.ok) {
      setMessage(`已创建备份：${r.data.backupPath}`)
      await refreshBackups()
    } else {
      setMessage(`${r.error.code} · ${r.error.message}`)
    }
  } finally {
    busy.value = false
  }
}

async function restoreBackup(path: string) {
  const ok = window.confirm('确认从该备份恢复？恢复会覆盖当前的 user.db（收藏、积分等个人数据）。')
  if (!ok) return
  busy.value = true
  try {
    const r = await appRestoreUserDb({ backupPath: path })
    if (r.ok) {
      setMessage('恢复完成')
      await app.refreshStatus()
      await refreshBackups()
    } else {
      setMessage(`${r.error.code} · ${r.error.message}`)
    }
  } finally {
    busy.value = false
  }
}

async function deleteBackup(path: string) {
  const ok = window.confirm('确认删除该备份文件？')
  if (!ok) return
  busy.value = true
  try {
    const r = await appDeleteBackup({ backupPath: path })
    if (r.ok) {
      setMessage('已删除备份')
      await refreshBackups()
    } else {
      setMessage(`${r.error.code} · ${r.error.message}`)
    }
  } finally {
    busy.value = false
  }
}

async function load() {
  if (!app.status) await app.refreshStatus()
  dataDir.value = await appGetDataDir()
  await refreshBackups()
}

onMounted(async () => {
  await load()
})

watch(open, async (v) => {
  if (!v) return
  message.value = ''
  await load()
})
</script>

<template>
  <div v-if="open" class="mask" role="dialog" aria-modal="true">
    <div class="card">
      <div class="head">
        <div class="title">设置</div>
        <button class="x" type="button" @click="close">×</button>
      </div>

      <div class="wrap">
        <div class="section">
          <div class="sectionHead">
            <div class="sectionTitle">本地数据</div>
          </div>
          <div class="row">
            <div class="k">数据目录</div>
            <div v-if="dataDir?.ok" class="v mono">{{ dataDir.data.path }}</div>
            <div v-else-if="dataDir && !dataDir.ok" class="v err">{{ dataDir.error.code }} · {{ dataDir.error.message }}</div>
            <div v-else class="v tute-muted">加载中…</div>
          </div>
          <div class="row">
            <div class="k">存储模式</div>
            <div v-if="dataDir?.ok" class="v">
              <span v-if="dataDir.data.mode === 'portable'">便携模式（数据写入 app.exe 同目录的 data/）</span>
              <span v-else>系统模式（数据写入 AppData）</span>
            </div>
            <div v-else class="v tute-muted">—</div>
          </div>
          <div class="hint">便携模式用法：将 app.exe 放到可写目录，并在同目录创建 data 文件夹（首次启动会自动生成数据库与 backups/）。</div>
        </div>

        <div class="section">
          <div class="sectionHead">
            <div class="sectionTitle">备份与恢复</div>
            <button class="tute-btn" type="button" :disabled="busy" @click="createBackup">{{ busy ? '处理中…' : '创建备份' }}</button>
          </div>
          <div class="hint">备份对象：user.db（收藏、答题积分等个人数据）</div>

          <div v-if="backups?.ok" class="list">
            <div v-if="backups.data.items.length === 0" class="empty tute-muted">暂无备份</div>
            <div v-for="b in backups.data.items" :key="b.path" class="item">
              <div class="main">
                <div class="name">{{ b.fileName }}</div>
                <div class="meta">
                  <span>{{ new Date(b.modifiedAt).toLocaleString() }}</span>
                  <span class="sep">·</span>
                  <span>{{ formatBytes(b.size) }}</span>
                </div>
              </div>
              <div class="actions">
                <button class="tute-btn-ghost" type="button" :disabled="busy" @click="restoreBackup(b.path)">恢复</button>
                <button class="tute-btn-ghost danger" type="button" :disabled="busy" @click="deleteBackup(b.path)">删除</button>
              </div>
            </div>
          </div>
          <div v-else-if="backups && !backups.ok" class="err">{{ backups.error.code }} · {{ backups.error.message }}</div>
          <div v-else class="tute-muted">加载中…</div>
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
  background: rgba(0, 0, 0, 0.4);
  display: grid;
  place-items: center;
  padding: 16px;
  z-index: 999;
}

.card {
  width: min(820px, 100%);
  max-height: min(86vh, 860px);
  overflow: auto;
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

.wrap {
  margin-top: 10px;
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

.v {
  margin-top: 6px;
  font-size: 12px;
  color: var(--text-primary);
}

.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
  word-break: break-all;
}

.err {
  margin-top: 10px;
  font-size: 12px;
  color: #d93026;
}

.list {
  margin-top: 12px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 6px;
  overflow: hidden;
}

.item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 12px;
  background: #ffffff;
  border-top: 1px solid rgba(0, 0, 0, 0.08);
}

.item:first-child {
  border-top: 0;
}

.name {
  font-weight: 800;
  font-size: 12px;
}

.meta {
  margin-top: 6px;
  font-size: 12px;
  color: var(--text-muted);
}

.sep {
  margin: 0 6px;
}

.actions {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.tute-btn-ghost.danger {
  border-color: rgba(217, 48, 38, 0.3);
  color: #d93026;
}

.tute-btn-ghost.danger:hover {
  background: rgba(217, 48, 38, 0.06);
  color: #d93026;
}

.empty {
  padding: 12px 12px;
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
</style>
