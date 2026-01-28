import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'
import TauriCallView from '../components/TauriCallView.vue'
import SettingsView from '../views/SettingsView.vue'

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    name: 'meeting',
    component: TauriCallView
  },
  {
    path: '/settings',
    name: 'settings',
    component: SettingsView
  }
]

const router = createRouter({
  history: createWebHashHistory(),
  routes
})

export default router
