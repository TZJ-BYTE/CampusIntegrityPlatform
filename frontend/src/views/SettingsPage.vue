<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useAppStore } from '../stores/app'
import {
  appBackupUserDb,
  appDeleteBackup,
  appGetDataDir,
  appListBackups,
  appRestoreUserDb,
  contentApplyPack,
  contentCheckUpdate,
  contentDownloadUpdate,
  contentImportDb,
  userGetSettings,
  userUpdateSettings,
  type ApiResponse,
  type BackupsList,
  type ContentCheckUpdateResult,
  type DataDirInfo,
} from '../api/tauri'

const app = useAppStore()
const dataDir = ref<ApiResponse<DataDirInfo> | null>(null)
const backups = ref<ApiResponse<BackupsList> | null>(null)
const busy = ref(false)
const message = ref('')
const contentSourcePath = ref('')
const packPath = ref('')
const updateBaseUrl = ref('')
const updateInfo = ref<ApiResponse<ContentCheckUpdateResult> | null>(null)
const downloadedPackPath = ref('')

async function refreshBackups() {
  backups.value = await appListBackups()
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

function setMessage(s: string) {
  message.value = s
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

async function importContentDb() {
  const p = contentSourcePath.value.trim()
  if (!p) {
    setMessage('请先填写内容库文件路径（.db 或 .sqlite）')
    return
  }
  const ok = window.confirm('确认导入内容库？导入会覆盖当前 content.db，但不会影响 user.db（收藏、积分等个人数据）。')
  if (!ok) return
  busy.value = true
  try {
    const r = await contentImportDb({ sourcePath: p })
    if (r.ok) {
      setMessage(`导入成功：内容版本 ${r.data.contentVersion}（已自动备份旧 content.db）`)
      await app.refreshStatus()
    } else {
      setMessage(`${r.error.code} · ${r.error.message}`)
    }
  } finally {
    busy.value = false
  }
}

async function applyPack() {
  const p = packPath.value.trim()
  if (!p) {
    setMessage('请先填写更新包路径（.zip）')
    return
  }
  const ok = window.confirm('确认导入更新包？导入会覆盖当前 content.db，但不会影响 user.db（收藏、积分等个人数据）。')
  if (!ok) return
  busy.value = true
  try {
    const r = await contentApplyPack({ packPath: p })
    if (r.ok) {
      setMessage(`导入成功：内容版本 ${r.data.newContentVersion}（已自动备份旧 content.db）`)
      await app.refreshStatus()
    } else {
      setMessage(`${r.error.code} · ${r.error.message}`)
    }
  } finally {
    busy.value = false
  }
}

async function loadSettings() {
  const r = await userGetSettings()
  if (r.ok) {
    updateBaseUrl.value = r.data.items.contentUpdateBaseUrl ?? ''
  }
}

async function saveUpdateBaseUrl() {
  const v = updateBaseUrl.value.trim()
  const r = await userUpdateSettings({ patch: { contentUpdateBaseUrl: v } })
  if (r.ok) {
    setMessage('已保存更新服务器地址')
  } else {
    setMessage(`${r.error.code} · ${r.error.message}`)
  }
}

async function checkUpdate() {
  const base = updateBaseUrl.value.trim()
  if (!base) {
    setMessage('请先填写更新服务器地址')
    return
  }
  busy.value = true
  try {
    updateInfo.value = await contentCheckUpdate({ baseUrl: base })
    downloadedPackPath.value = ''
    if (updateInfo.value.ok) {
      if (updateInfo.value.data.hasUpdate) {
        setMessage(`发现新内容版本：${updateInfo.value.data.latestVersion}`)
      } else {
        setMessage('已是最新内容版本')
      }
    } else {
      setMessage(`${updateInfo.value.error.code} · ${updateInfo.value.error.message}`)
    }
  } finally {
    busy.value = false
  }
}

async function downloadAndApply() {
  if (!updateInfo.value?.ok) return
  if (!updateInfo.value.data.hasUpdate) return
  const ok = window.confirm('确认下载并更新内容库？更新前会自动备份当前 content.db。')
  if (!ok) return
  busy.value = true
  try {
    const d = await contentDownloadUpdate({ url: updateInfo.value.data.downloadUrl })
    if (!d.ok) {
      setMessage(`${d.error.code} · ${d.error.message}`)
      return
    }
    downloadedPackPath.value = d.data.packPath
    const a = await contentApplyPack({ packPath: d.data.packPath })
    if (a.ok) {
      setMessage(`更新完成：内容版本 ${a.data.newContentVersion}`)
      await app.refreshStatus()
    } else {
      setMessage(`${a.error.code} · ${a.error.message}`)
    }
  } finally {
    busy.value = false
  }
}

onMounted(async () => {
  if (!app.status) await app.refreshStatus()
  dataDir.value = await appGetDataDir()
  await refreshBackups()
  await loadSettings()
})
</script>

<template>
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
      <div class="hint">
        便携模式用法：将 app.exe 放到可写目录，并在同目录创建 data 文件夹（首次启动会自动生成数据库与 backups/）。
      </div>
    </div>

    <div class="section">
      <div class="sectionHead">
        <div class="sectionTitle">备份与恢复</div>
        <button class="tute-btn" type="button" :disabled="busy" @click="createBackup">
          {{ busy ? '处理中…' : '创建备份' }}
        </button>
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

    <div class="section">
      <div class="sectionHead">
        <div class="sectionTitle">内容库管理</div>
        <button class="tute-btn" type="button" :disabled="busy" @click="importContentDb">
          {{ busy ? '处理中…' : '导入内容库' }}
        </button>
      </div>
      <div class="hint">
        当前内容版本：<span v-if="app.status?.ok">{{ app.status.data.contentVersion }}</span><span v-else>—</span>
      </div>
      <div class="row">
        <div class="k">内容库文件路径</div>
        <input v-model="contentSourcePath" class="tute-input input" placeholder="例如：D:\content.db" />
        <div class="tip">建议使用由 SQLite “VACUUM INTO” 导出的单文件内容库，避免携带 -wal/-shm 导致不完整。</div>
      </div>
    </div>

    <div class="section">
      <div class="sectionHead">
        <div class="sectionTitle">内容更新包</div>
        <button class="tute-btn" type="button" :disabled="busy" @click="applyPack">
          {{ busy ? '处理中…' : '导入更新包' }}
        </button>
      </div>
      <div class="hint">更新包格式：zip（包含 manifest.json 与 content.db）</div>
      <div class="row">
        <div class="k">更新包路径</div>
        <input v-model="packPath" class="tute-input input" placeholder="例如：D:\content-pack.zip" />
        <div class="tip">导入前会自动备份当前 content.db 到 backups/。</div>
      </div>
    </div>

    <div class="section">
      <div class="sectionHead">
        <div class="sectionTitle">联机内容更新</div>
        <button class="tute-btn-ghost" type="button" :disabled="busy" @click="saveUpdateBaseUrl">保存地址</button>
      </div>
      <div class="hint">用于自动检查内容版本与下载更新包（仍会本地备份并应用）。</div>
      <div class="row">
        <div class="k">更新服务器地址</div>
        <input v-model="updateBaseUrl" class="tute-input input" placeholder="例如：http://127.0.0.1:8787" />
        <div class="actionsInline">
          <button class="tute-btn-ghost" type="button" :disabled="busy" @click="checkUpdate">检查更新</button>
          <button
            class="tute-btn"
            type="button"
            :disabled="busy || !updateInfo?.ok || !updateInfo.data.hasUpdate"
            @click="downloadAndApply"
          >
            下载并更新
          </button>
        </div>
        <div v-if="updateInfo?.ok" class="tip">
          <span v-if="updateInfo.data.hasUpdate">最新版本：{{ updateInfo.data.latestVersion }}</span>
          <span v-else>当前已是最新：{{ updateInfo.data.latestVersion }}</span>
          <span v-if="updateInfo.data.notes"> · {{ updateInfo.data.notes }}</span>
        </div>
        <div v-else-if="updateInfo && !updateInfo.ok" class="tip">{{ updateInfo.error.code }} · {{ updateInfo.error.message }}</div>
        <div v-if="downloadedPackPath" class="tip">已下载：{{ downloadedPackPath }}</div>
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
  padding: 10px 12px;
  border-radius: 6px;
  background: rgba(139, 26, 92, 0.06);
  border: 1px solid rgba(139, 26, 92, 0.18);
  color: var(--text-secondary);
  font-size: 12px;
}
</style>
