<script setup lang="ts">
const props = defineProps<{
  badge?: string
  meta?: string
  title: string
  subtitle?: string
  clickable?: boolean
  thumb?: string
  thumbAlt?: string
}>()

const emit = defineEmits<{ (e: 'click'): void }>()

function onClick() {
  if (!props.clickable) return
  emit('click')
}
</script>

<template>
  <div class="card tute-card" :class="{ clickable: !!props.clickable }" @click="onClick">
    <div class="top">
      <div class="left">
        <span v-if="props.badge" class="badge">{{ props.badge }}</span>
        <span v-if="props.meta" class="meta">{{ props.meta }}</span>
      </div>
      <div class="right">
        <slot name="topRight" />
      </div>
    </div>

    <div class="mid" :class="{ withThumb: !!props.thumb }">
      <div class="midMain">
        <div class="title">{{ props.title }}</div>
        <div v-if="props.subtitle" class="subtitle">{{ props.subtitle }}</div>
      </div>
      <img v-if="props.thumb" class="thumb" :src="props.thumb" :alt="props.thumbAlt || props.title" />
    </div>

    <div class="body">
      <slot />
    </div>

    <div v-if="$slots.footer" class="footer">
      <slot name="footer" />
    </div>
  </div>
</template>

<style scoped>
.card {
  padding: 12px 12px;
  border-radius: 10px;
  border: 1px solid rgba(0, 0, 0, 0.06);
  transition: transform 160ms ease-out, box-shadow 160ms ease-out, border-color 160ms ease-out;
}

.clickable {
  cursor: pointer;
}

.clickable:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-card-hover);
  border-color: rgba(139, 26, 92, 0.14);
}

.clickable:active {
  transform: translateY(-1px) scale(0.995);
}

.top {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  align-items: center;
}

.left {
  display: inline-flex;
  gap: 8px;
  align-items: center;
}

.badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border-radius: 999px;
  background: rgba(139, 26, 92, 0.08);
  border: 1px solid rgba(139, 26, 92, 0.18);
  color: var(--primary);
  font-size: 12px;
  font-weight: 800;
}

.meta {
  font-size: 12px;
  color: var(--text-muted);
}

.mid {
  margin-top: 8px;
  display: grid;
  gap: 10px;
}

.mid.withThumb {
  grid-template-columns: 1fr 140px;
  align-items: start;
}

.midMain {
  min-width: 0;
}

.title {
  font-size: 15px;
  font-weight: 900;
  line-height: 1.25;
}

.subtitle {
  margin-top: 6px;
  font-size: 12px;
  color: var(--text-secondary);
}

.thumb {
  width: 140px;
  height: 86px;
  border-radius: 10px;
  object-fit: cover;
  border: 1px solid rgba(0, 0, 0, 0.08);
}

.body {
  margin-top: 10px;
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.7;
  white-space: pre-wrap;
}

.footer {
  margin-top: 10px;
  display: flex;
  justify-content: space-between;
  gap: 10px;
  align-items: center;
}
</style>
