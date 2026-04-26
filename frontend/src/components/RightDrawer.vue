<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import { animateIn, animateOut, animateStaggerIn } from '../utils/motion'

const props = defineProps<{
  open: boolean
  title?: string
}>()

const emit = defineEmits<{ (e: 'close'): void }>()

const shown = ref(false)
const maskEl = ref<HTMLElement | null>(null)
const panelEl = ref<HTMLElement | null>(null)

function close() {
  emit('close')
}

watch(
  () => props.open,
  async (v) => {
    if (v) {
      shown.value = true
      await nextTick()
      if (maskEl.value) {
        animateIn(maskEl.value, { from: { opacity: 0, y: 0 }, to: { opacity: 1, y: 0, duration: 0.22 } })
      }
      if (panelEl.value) {
        animateIn(panelEl.value, { from: { x: 24, y: 0, opacity: 0.92, scale: 0.995 }, to: { x: 0, y: 0, opacity: 1, scale: 1 } })
        animateStaggerIn(panelEl.value, '.body > *', { from: { opacity: 0, y: 10 }, to: { opacity: 1, y: 0, duration: 0.34 }, stagger: 0.04 })
      }
      return
    }

    if (!shown.value) return
    await Promise.all([
      panelEl.value ? animateOut(panelEl.value, { x: 24, y: 0, opacity: 0 }) : Promise.resolve(),
      maskEl.value ? animateOut(maskEl.value, { opacity: 0, y: 0, duration: 0.18 }) : Promise.resolve(),
    ])
    shown.value = false
  },
  { immediate: true },
)
</script>

<template>
  <div v-if="shown" ref="maskEl" class="mask" role="dialog" aria-modal="true" @click.self="close">
    <div ref="panelEl" class="panel">
      <div v-if="props.title" class="top">
        <div class="title">{{ props.title }}</div>
        <button class="tute-btn-ghost" type="button" @click="close">关闭</button>
      </div>
      <div class="body">
        <slot />
      </div>
    </div>
  </div>
</template>

<style scoped>
.mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.35);
  z-index: 1050;
  display: flex;
  justify-content: flex-end;
}

.panel {
  width: min(640px, 92vw);
  height: 100%;
  background: var(--bg-card);
  border-left: 1px solid rgba(0, 0, 0, 0.08);
  box-shadow: -18px 0 60px rgba(0, 0, 0, 0.22);
  display: flex;
  flex-direction: column;
  will-change: transform, opacity;
}

.top {
  padding: 12px 12px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.08);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.title {
  font-weight: 900;
  font-size: 14px;
}

.body {
  flex: 1;
  overflow: auto;
  padding: 14px 14px 18px;
}

</style>
