import { createRouter, createWebHashHistory } from 'vue-router'

import HomePage from '../views/HomePage.vue'
import VenuesPage from '../views/VenuesPage.vue'
import CasesPage from '../views/CasesPage.vue'
import RegulationsPage from '../views/RegulationsPage.vue'
import StoriesPage from '../views/StoriesPage.vue'
import VenueDetailPage from '../views/VenueDetailPage.vue'
import CaseDetailPage from '../views/CaseDetailPage.vue'
import RegulationDetailPage from '../views/RegulationDetailPage.vue'
import StoryDetailPage from '../views/StoryDetailPage.vue'
import QuizPage from '../views/QuizPage.vue'
import FavoritesPage from '../views/FavoritesPage.vue'

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', name: 'home', component: HomePage, meta: { section: '廉洁文化', title: '首页' } },
    { path: '/venues', name: 'venues', component: VenuesPage, meta: { section: '廉洁文化', title: '廉洁文化场所' } },
    { path: '/venues/:id', name: 'venueDetail', component: VenueDetailPage, meta: { section: '廉洁文化', title: '场所详情' } },
    { path: '/cases', name: 'cases', component: CasesPage, meta: { section: '廉洁文化', title: '身边违纪行为警示' } },
    { path: '/cases/:id', name: 'caseDetail', component: CaseDetailPage, meta: { section: '廉洁文化', title: '案例详情' } },
    { path: '/regulations', name: 'regulations', component: RegulationsPage, meta: { section: '廉洁文化', title: '政策法规学习' } },
    { path: '/regulations/:id', name: 'regulationDetail', component: RegulationDetailPage, meta: { section: '廉洁文化', title: '法规详情' } },
    { path: '/stories', name: 'stories', component: StoriesPage, meta: { section: '廉洁文化', title: '每日廉洁故事' } },
    { path: '/stories/:id', name: 'storyDetail', component: StoryDetailPage, meta: { section: '廉洁文化', title: '故事详情' } },
    { path: '/quiz', name: 'quiz', component: QuizPage, meta: { section: '廉洁文化', title: '廉洁知识竞答' } },
    { path: '/favorites', name: 'favorites', component: FavoritesPage, meta: { section: '廉洁文化', title: '我的收藏' } },
  ],
})
