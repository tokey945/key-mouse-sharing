import { createMemoryHistory, createRouter } from 'vue-router'

import FileSharing from '@/pages/FileSharing.vue'
import KeyMouseSharing from '@/pages/KeyMouseSharing.vue'
import Settings from '@/pages/Settings.vue'
import DeveloperTools from '@/pages/DeveloperTools.vue'
import Toolbox from '@/pages/Toolbox.vue'

// 当前应用是桌面端（Tauri），使用内存路由避免依赖浏览器 URL。
const routes = [
  { path: '/', component: KeyMouseSharing },
  { path: '/file/sharing', component: FileSharing },
  { path: '/settings', component: Settings },
  { path: '/developer/tools', component: DeveloperTools },
  { path: '/toolbox', component: Toolbox },
]

const router = createRouter({
  history: createMemoryHistory(),
  routes,
})
export default router
