<script setup lang="ts">
const props = defineProps<{
  open: boolean
  title: string
  message: string
  confirmText?: string
  cancelText?: string
  danger?: boolean
}>()

const emit = defineEmits<{ (e: 'confirm'): void; (e: 'cancel'): void }>()

function cancel() {
  emit('cancel')
}

function confirm() {
  emit('confirm')
}
</script>

<template>
  <div v-if="props.open" class="mask" role="dialog" aria-modal="true">
    <div class="card" @click.stop>
      <div class="title">{{ props.title }}</div>
      <div class="msg">{{ props.message }}</div>
      <div class="actions">
        <button class="tute-btn-ghost" type="button" @click="cancel">{{ props.cancelText ?? '取消' }}</button>
        <button class="tute-btn" :class="{ danger: !!props.danger }" type="button" @click="confirm">
          {{ props.confirmText ?? '确认' }}
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
  z-index: 1100;
}

.card {
  width: min(520px, 100%);
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 10px;
  background: #ffffff;
  padding: 14px 14px;
  box-shadow: 0 18px 60px rgba(0, 0, 0, 0.22);
}

.title {
  font-weight: 900;
  font-size: 14px;
  color: var(--text-primary);
}

.msg {
  margin-top: 10px;
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.6;
  white-space: pre-wrap;
}

.actions {
  margin-top: 12px;
  display: flex;
  gap: 10px;
  justify-content: flex-end;
}

.danger {
  border-color: rgba(217, 48, 38, 0.3) !important;
  background: rgba(217, 48, 38, 0.92) !important;
}
</style>

