<template>
  <div class="layout">
    <main class="main">
      <slot name="main" />
    </main>
    <aside class="aside">
      <slot name="aside" />
    </aside>
  </div>
</template>

<style scoped>
.layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 320px;
  gap: 16px;
  align-items: start;
}

.aside {
  position: sticky;
  top: 84px;
  display: grid;
  gap: 12px;
}

@media (max-width: 980px) {
  .layout {
    grid-template-columns: minmax(0, 1fr);
  }
  .aside {
    position: static;
  }
}
</style>

