<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useAppStore } from '../stores/app'
import { useProfileStore } from '../stores/profile'
import { useSyncStore } from '../stores/sync'
import { useUiStore } from '../stores/ui'
import AvatarCircle from './AvatarCircle.vue'

const ui = useUiStore()
const app = useAppStore()
const sync = useSyncStore()
const profile = useProfileStore()

const open = computed(() => ui.overlay === 'profile')
const message = ref('')

const username = computed(() => (sync.auth?.ok ? (sync.auth.data.username ?? '').trim() : ''))
const deviceId = computed(() => (app.status?.ok ? app.status.data.deviceId : ''))
const seed = computed(() => username.value || profile.profile.nickname.trim() || deviceId.value || 'cip')

const nickname = ref('')
const avatarColor = ref('')

watch(
  open,
  async (v) => {
    if (!v) return
    message.value = ''
    await profile.loadOnce()
    if (!app.status) await app.refreshStatus()
    await sync.refresh()
    nickname.value = profile.profile.nickname
    avatarColor.value = profile.profile.avatarColor
  },
  { immediate: false },
)

function close() {
  ui.close()
}

async function save() {
  await profile.setNickname(nickname.value)
  await profile.setAvatarColor(avatarColor.value)
  message.value = '已保存'
}

async function onPickImage(e: Event) {
  const input = e.target as HTMLInputElement
  const f = input.files?.[0]
  input.value = ''
  if (!f) return
  if (f.size > 256 * 1024) {
    message.value = '图片过大，请选择小于 256KB 的图片'
    return
  }
  const dataUrl: string = await new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(String(reader.result || ''))
    reader.onerror = () => reject(new Error('read_failed'))
    reader.readAsDataURL(f)
  })
  if (!dataUrl.startsWith('data:image/')) {
    message.value = '文件格式不支持'
    return
  }
  await profile.setAvatarImageDataUrl(dataUrl)
  message.value = '头像已更新'
}

async function clearImage() {
  await profile.clearAvatarImage()
  message.value = '已移除头像图片'
}
</script>

<template>
  <div v-if="open" class="mask" role="dialog" aria-modal="true">
    <div class="card">
      <div class="head">
        <div class="title">用户信息</div>
        <button class="x" type="button" @click="close">×</button>
      </div>

      <div class="row">
        <div class="k">头像预览</div>
        <div class="avatarRow">
          <AvatarCircle
            :size="56"
            :seed="seed"
            :label="nickname.trim() || username || '离线'"
            :baseColor="avatarColor"
            :imageDataUrl="profile.profile.avatarImageDataUrl || null"
          />
          <div class="avatarActions">
            <label class="tute-btn-ghost fileBtn">
              上传图片
              <input class="fileInput" type="file" accept="image/*" @change="onPickImage" />
            </label>
            <button class="tute-btn-ghost" type="button" @click="clearImage">移除图片</button>
          </div>
        </div>
        <div class="hint">建议使用较小图片（≤256KB）。未上传时使用渐变头像。</div>
      </div>

      <div class="row">
        <div class="k">昵称</div>
        <input v-model="nickname" class="tute-input input" placeholder="可选，用于本机显示" />
      </div>

      <div class="row">
        <div class="k">头像主色</div>
        <div class="colorRow">
          <input v-model="avatarColor" class="color" type="color" />
          <input v-model="avatarColor" class="tute-input input" placeholder="#RRGGBB（可留空自动生成）" />
        </div>
      </div>

      <div v-if="message" class="message">{{ message }}</div>

      <div class="actions">
        <button class="tute-btn-ghost" type="button" @click="close">关闭</button>
        <button class="tute-btn" type="button" @click="save">保存</button>
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
  width: min(640px, 100%);
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

.hint {
  margin-top: 10px;
  font-size: 12px;
  color: var(--text-muted);
}

.input {
  margin-top: 8px;
  width: 100%;
}

.avatarRow {
  margin-top: 10px;
  display: flex;
  gap: 12px;
  align-items: center;
}

.avatarActions {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.fileBtn {
  cursor: pointer;
}

.fileInput {
  display: none;
}

.colorRow {
  margin-top: 10px;
  display: grid;
  grid-template-columns: 52px 1fr;
  gap: 10px;
  align-items: center;
}

.color {
  width: 52px;
  height: 36px;
  border: 0;
  padding: 0;
  background: transparent;
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

.actions {
  margin-top: 12px;
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  justify-content: flex-end;
}
</style>
