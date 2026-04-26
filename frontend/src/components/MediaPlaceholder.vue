<script setup lang="ts">
import coverUrl from '../assets/placeholder-cover.svg'

const props = defineProps<{
  kind: 'image' | 'video'
  label?: string
  ratio?: string
}>()

const ratio = props.ratio ?? '16 / 9'
</script>

<template>
  <div class="wrap" :style="{ aspectRatio: ratio }">
    <img class="img" :src="coverUrl" alt="占位" />
    <div class="overlay">
      <div class="badge">{{ props.kind === 'video' ? '视频' : '图片' }}</div>
      <div class="text">{{ props.label ?? (props.kind === 'video' ? '视频占位' : '图片占位') }}</div>
      <div v-if="props.kind === 'video'" class="play">
        <div class="tri"></div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.wrap {
  position: relative;
  width: 100%;
  border-radius: 12px;
  overflow: hidden;
  border: 1px solid rgba(0, 0, 0, 0.08);
  background: #ffffff;
}

.img {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  transform: scale(1.02);
  filter: saturate(1.02);
}

.overlay {
  position: absolute;
  inset: 0;
  display: grid;
  align-content: end;
  gap: 10px;
  padding: 12px 12px;
  background: linear-gradient(180deg, rgba(0, 0, 0, 0.02), rgba(0, 0, 0, 0.35));
}

.badge {
  justify-self: start;
  background: rgba(255, 255, 255, 0.9);
  border: 1px solid rgba(0, 0, 0, 0.08);
  color: var(--text-primary);
  border-radius: 999px;
  padding: 4px 10px;
  font-size: 12px;
  font-weight: 900;
}

.text {
  color: rgba(255, 255, 255, 0.92);
  font-size: 12px;
  font-weight: 800;
  text-shadow: 0 2px 10px rgba(0, 0, 0, 0.22);
}

.play {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
}

.tri {
  width: 0;
  height: 0;
  border-left: 18px solid rgba(255, 255, 255, 0.92);
  border-top: 12px solid transparent;
  border-bottom: 12px solid transparent;
  filter: drop-shadow(0 8px 16px rgba(0, 0, 0, 0.28));
  transform: translateX(2px);
}
</style>
