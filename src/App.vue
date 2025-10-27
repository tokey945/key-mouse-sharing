<script setup lang="ts">
import { SidebarProvider, SidebarTrigger } from '@/components/ui/sidebar'
import AppSidebar from '@/components/AppSidebar.vue'
import ColorMode from '@/components/ColorMode.vue'
</script>

<template>
  <SidebarProvider>
    <AppSidebar />
    <main class="w-full h-screen flex flex-col">
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
