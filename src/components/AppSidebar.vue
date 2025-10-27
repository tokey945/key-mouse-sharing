<script setup lang="ts">
import { Keyboard, CloudUpload, ScreenShare } from 'lucide-vue-next'
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem
} from '@/components/ui/sidebar'
import { useRoute } from 'vue-router'

const route = useRoute()
// Menu items.
const items = [
  {
    title: '键鼠共享',
    url: '/',
    icon: Keyboard
  },
  {
    title: '文件共享',
    url: '/file/sharing',
    icon: CloudUpload
  },
  {
    title: '屏幕共享',
    url: '/screen/sharing',
    icon: ScreenShare
  }
]
</script>

<template>
  <Sidebar collapsible="icon">
    <SidebarContent data-tauri-drag-region>
      <SidebarGroup>
        <SidebarGroupContent>
          <SidebarMenu class="pt-6">
            <SidebarMenuItem v-for="item in items" :key="item.title">
              <SidebarMenuButton
                asChild
                :class="[
                  'transition-colors',
                  'hover:bg-accent',
                  route.path === item.url ? 'bg-accent' : 'active:bg-accent',
                ]"
              >
                <RouterLink :to="item.url">
                  <component :is="item.icon" />
                  <span class="group-data-[collapsible=icon]:hidden">{{ item.title }}</span>
                </RouterLink>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>
    </SidebarContent>
  </Sidebar>
</template>
