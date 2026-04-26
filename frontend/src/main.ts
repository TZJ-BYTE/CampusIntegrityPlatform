import { createApp } from 'vue'
import './style.css'
import App from './App.vue'
import { createPinia } from 'pinia'
import { router } from './router'
import { useSyncStore } from './stores/sync'
import { useContentUpdateStore } from './stores/contentUpdate'

const app = createApp(App)
const pinia = createPinia()
app.use(pinia).use(router).mount('#app')

useSyncStore(pinia).startAuto()
useContentUpdateStore(pinia).startAuto()
