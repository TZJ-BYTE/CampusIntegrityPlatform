import { createRouter, createWebHashHistory } from 'vue-router'

import HomePage from '../views/HomePage.vue'
import VenuesPage from '../views/VenuesPage.vue'
import CasesPage from '../views/CasesPage.vue'
import RegulationsPage from '../views/RegulationsPage.vue'
import StoriesPage from '../views/StoriesPage.vue'
import QuizPage from '../views/QuizPage.vue'
import FavoritesPage from '../views/FavoritesPage.vue'
import SettingsPage from '../views/SettingsPage.vue'

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', name: 'home', component: HomePage, meta: { section: '廉洁文化', title: '应用系统' } },
    { path: '/venues', name: 'venues', component: VenuesPage, meta: { section: '廉洁文化', title: '廉洁文化场所' } },
    { path: '/cases', name: 'cases', component: CasesPage, meta: { section: '廉洁文化', title: '身边违纪行为警示' } },
    { path: '/regulations', name: 'regulations', component: RegulationsPage, meta: { section: '廉洁文化', title: '政策法规学习' } },
    { path: '/stories', name: 'stories', component: StoriesPage, meta: { section: '廉洁文化', title: '每日廉洁故事' } },
    { path: '/quiz', name: 'quiz', component: QuizPage, meta: { section: '廉洁文化', title: '廉洁知识竞答' } },
    { path: '/favorites', name: 'favorites', component: FavoritesPage, meta: { section: '廉洁文化', title: '我的收藏' } },
    { path: '/settings', name: 'settings', component: SettingsPage, meta: { section: '廉洁文化', title: '设置' } },
  ],
})
