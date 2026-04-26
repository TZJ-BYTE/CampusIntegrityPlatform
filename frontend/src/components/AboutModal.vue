<script setup lang="ts">
import { computed, watch } from 'vue'
import { useAppStore } from '../stores/app'
import { useUiStore } from '../stores/ui'

const ui = useUiStore()
const app = useAppStore()
const open = computed(() => ui.overlay === 'about')

watch(
  open,
  async (v) => {
    if (!v) return
    if (!app.status) await app.refreshStatus()
  },
  { immediate: false },
)

function close() {
  ui.close()
}
</script>

<template>
  <div v-if="open" class="mask" role="dialog" aria-modal="true">
    <div class="card">
      <div class="head">
        <div class="title">关于</div>
        <button class="x" type="button" @click="close">×</button>
      </div>

      <div class="row">
        <div class="k">应用信息</div>
        <div class="tip" v-if="app.status?.ok">
          <div>版本：v{{ app.status.data.appVersion }}</div>
          <div>内容版本：{{ app.status.data.contentVersion }}</div>
          <div>设备标识：{{ app.status.data.deviceId }}</div>
        </div>
        <div class="tip" v-else-if="app.status && !app.status.ok">读取失败：{{ app.status.error.code }} · {{ app.status.error.message }}</div>
        <div class="tip" v-else>加载中…</div>
      </div>

      <div class="actions">
        <button class="tute-btn-ghost" type="button" @click="close">关闭</button>
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
  width: min(560px, 100%);
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

.tip {
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.6;
}

.actions {
  margin-top: 12px;
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  justify-content: flex-end;
}
</style>
