<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { SidebarProvider, SidebarTrigger } from '@/components/ui/sidebar'
import AppSidebar from '@/components/AppSidebar.vue'
import ColorMode from '@/components/ColorMode.vue'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import { setConnectionState, type ConnectionState } from '@/composables/useConnectionState'
import { useAppSettings } from '@/composables/useAppSettings'

interface DeviceTrustRequest {
  requestId: string
  deviceId: string
  deviceName: string
  peerIp: string
}

const settings = useAppSettings()
const trustRequest = ref<DeviceTrustRequest | null>(null)
const trustDialogOpen = computed(() => Boolean(trustRequest.value))
let unlistenTrust: UnlistenFn | null = null
let unlistenConnection: UnlistenFn | null = null

const isTauriRuntime = () =>
  typeof window !== 'undefined' &&
  typeof (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ === 'object'

const answerTrust = async (decision: 'allowOnce' | 'allowRemember' | 'deny') => {
  const request = trustRequest.value
  if (!request) return

  if (decision === 'allowRemember') {
    settings.trustDevice({
      deviceId: request.deviceId,
      deviceName: request.deviceName,
      lastIp: request.peerIp,
    })
  }

  await invoke('answer_device_trust', {
    requestId: request.requestId,
    decision,
  })
  trustRequest.value = null
}

onMounted(async () => {
  if (!isTauriRuntime()) return

  unlistenTrust = await listen<DeviceTrustRequest>('device-trust-request', (event) => {
    trustRequest.value = event.payload
  })
  unlistenConnection = await listen<ConnectionState>('kms-connection-state', (event) => {
    setConnectionState(event.payload)
  })

  try {
    const state = await invoke<ConnectionState>('get_connection_state')
    setConnectionState(state)
  } catch {
    setConnectionState({ connected: false })
  }
})

onUnmounted(() => {
  unlistenTrust?.()
  unlistenConnection?.()
})
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

  <AlertDialog :open="trustDialogOpen">
    <AlertDialogContent>
      <AlertDialogHeader>
        <AlertDialogTitle>允许设备连接？</AlertDialogTitle>
        <AlertDialogDescription>
          {{ trustRequest?.deviceName || '未知设备' }} 正在从 {{ trustRequest?.peerIp || '未知 IP' }} 请求连接。
        </AlertDialogDescription>
      </AlertDialogHeader>
      <div class="rounded-md border bg-muted/40 p-3 text-xs text-muted-foreground">
        设备 ID：{{ trustRequest?.deviceId }}
      </div>
      <AlertDialogFooter>
        <AlertDialogCancel @click="answerTrust('deny')">拒绝</AlertDialogCancel>
        <Button variant="outline" @click="answerTrust('allowOnce')">允许一次</Button>
        <AlertDialogAction @click="answerTrust('allowRemember')">允许并记住</AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</template>
