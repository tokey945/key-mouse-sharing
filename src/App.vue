<script setup lang="ts">
import { SidebarProvider, SidebarTrigger } from '@/components/ui/sidebar'
import AppSidebar from '@/components/AppSidebar.vue'
import ColorMode from '@/components/ColorMode.vue'
</script>

<template>
  <!-- 应用壳层：统一侧边栏 + 顶部工具区，业务页面通过 RouterView 切换 -->
  <SidebarProvider>
    <AppSidebar />
    <main class="w-full h-screen flex flex-col bg-muted/30">
      <!-- data-tauri-drag-region 允许在自定义标题栏区域拖动窗口 -->
      <div data-tauri-drag-region class="h-11 px-4 flex items-center justify-between border-b bg-background/80">
        <SidebarTrigger />
        <ColorMode />
      </div>
      <div class="flex-1 min-h-0">
        <RouterView />
      </div>
    </main>
  </SidebarProvider>
</template>
