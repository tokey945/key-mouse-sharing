<script setup lang="ts">
import { SidebarProvider, SidebarTrigger } from '@/components/ui/sidebar'
import AppSidebar from '@/components/AppSidebar.vue'
import ColorMode from '@/components/ColorMode.vue'
</script>

<template>
  <!-- 应用壳层：统一侧边栏 + 顶部工具区，业务页面通过 RouterView 切换 -->
  <SidebarProvider>
    <AppSidebar />
    <main class="w-full h-screen flex flex-col">
      <!-- data-tauri-drag-region 允许在自定义标题栏区域拖动窗口 -->
      <div data-tauri-drag-region class="px-6 flex items-center justify-between">
        <SidebarTrigger />
        <ColorMode />
      </div>
      <div class="flex-1 min-h-0 pl-4">
        <RouterView />
      </div>
    </main>
  </SidebarProvider>
</template>
