<script setup lang="ts">
import { useToastStore } from '../stores/toast'

const toast = useToastStore()
</script>

<template>
  <div class="host" aria-live="polite">
    <div v-for="t in toast.items" :key="t.id" class="toast" :class="t.kind" @click="toast.remove(t.id)">
      {{ t.message }}
    </div>
  </div>
</template>

<style scoped>
.host {
  position: fixed;
  right: 16px;
  bottom: 16px;
  display: grid;
  gap: 10px;
  z-index: 1200;
  max-width: min(520px, 92vw);
}

.toast {
  padding: 10px 12px;
  border-radius: 10px;
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.1);
  box-shadow: 0 18px 60px rgba(0, 0, 0, 0.18);
  font-size: 12px;
  color: var(--text-primary);
  cursor: pointer;
  user-select: none;
  line-height: 1.5;
}

.info {
  border-color: rgba(139, 26, 92, 0.22);
  background: rgba(139, 26, 92, 0.06);
}

.success {
  border-color: rgba(30, 142, 62, 0.22);
  background: rgba(30, 142, 62, 0.06);
}

.error {
  border-color: rgba(217, 48, 38, 0.22);
  background: rgba(217, 48, 38, 0.06);
}
</style>

