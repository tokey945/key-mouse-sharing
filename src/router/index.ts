import { createMemoryHistory, createRouter } from 'vue-router'

import FileSharing from '@/pages/FileSharing.vue'
import KeyMouseSharing from '@/pages/KeyMouseSharing.vue'
import ScreenSharing from '@/pages/ScreenSharing.vue'

// 当前应用是桌面端（Tauri），使用内存路由避免依赖浏览器 URL。
const routes = [
  { path: '/', component: KeyMouseSharing },
  { path: '/file/sharing', component: FileSharing },
  { path: '/screen/sharing', component: ScreenSharing },
]

const router = createRouter({
  history: createMemoryHistory(),
  routes,
})
export default router
