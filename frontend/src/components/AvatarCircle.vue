<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  size?: number
  imageDataUrl?: string | null
  seed?: string | null
  baseColor?: string | null
  label?: string | null
}>()

function clamp(n: number, min: number, max: number) {
  return Math.max(min, Math.min(max, n))
}

function hash32(s: string) {
  let h = 2166136261
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i)
    h = Math.imul(h, 16777619)
  }
  return h >>> 0
}

function hslToHex(h: number, s: number, l: number) {
  s /= 100
  l /= 100
  const c = (1 - Math.abs(2 * l - 1)) * s
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1))
  const m = l - c / 2
  let r = 0
  let g = 0
  let b = 0
  if (h < 60) [r, g, b] = [c, x, 0]
  else if (h < 120) [r, g, b] = [x, c, 0]
  else if (h < 180) [r, g, b] = [0, c, x]
  else if (h < 240) [r, g, b] = [0, x, c]
  else if (h < 300) [r, g, b] = [x, 0, c]
  else [r, g, b] = [c, 0, x]
  const toHex = (v: number) => Math.round((v + m) * 255).toString(16).padStart(2, '0')
  return `#${toHex(r)}${toHex(g)}${toHex(b)}`
}

const sizePx = computed(() => clamp(props.size ?? 32, 20, 72))

const effectiveSeed = computed(() => {
  const s = (props.seed ?? '').trim()
  if (s) return s
  const l = (props.label ?? '').trim()
  if (l) return l
  return 'cip'
})

const autoColor = computed(() => {
  const base = (props.baseColor ?? '').trim()
  if (base) return base
  const h = hash32(effectiveSeed.value) % 360
  return hslToHex(h, 62, 42)
})

const gradient = computed(() => {
  const h = hash32(`${effectiveSeed.value}|g`) % 360
  const c2 = hslToHex(h, 70, 38)
  const c1 = '#ffffff'
  return `radial-gradient(circle at 30% 28%, ${c1} 0%, ${c1} 18%, ${autoColor.value} 52%, ${c2} 100%)`
})

const initials = computed(() => {
  const t = (props.label ?? '').trim()
  if (!t) return '廉'
  const first = t[0]
  return first || '廉'
})
</script>

<template>
  <div
    class="avatar"
    :style="{
      width: `${sizePx}px`,
      height: `${sizePx}px`,
      backgroundImage: props.imageDataUrl ? `url(${props.imageDataUrl})` : gradient,
    }"
  >
    <div v-if="!props.imageDataUrl" class="text">{{ initials }}</div>
  </div>
</template>

<style scoped>
.avatar {
  border-radius: 999px;
  display: grid;
  place-items: center;
  overflow: hidden;
  background-size: cover;
  background-position: center;
  border: 1px solid rgba(255, 255, 255, 0.28);
}

.text {
  font-weight: 900;
  font-size: 12px;
  color: rgba(0, 0, 0, 0.72);
  text-shadow: 0 1px 1px rgba(255, 255, 255, 0.45);
  user-select: none;
}
</style>
