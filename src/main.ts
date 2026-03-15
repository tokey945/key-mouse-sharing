import { createApp } from 'vue'
import App from './App.vue'
import '@/assets/index.css'
import router from '@/router/index'

// 前端入口：挂载根组件并注入路由。
createApp(App).use(router).mount('#app')
