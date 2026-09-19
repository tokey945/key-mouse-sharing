<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { gsap } from 'gsap'
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
import { connectionState, setConnectionState, type ConnectionState } from '@/composables/useConnectionState'
import { useAppSettings } from '@/composables/useAppSettings'
import {
  pruneLanDevices,
  upsertLanDevice,
  useLanDiscovery,
  type LanConnectRequest,
  type LanConnectResponse,
  type LanDevice,
} from '@/composables/useLanDiscovery'

interface DeviceTrustRequest {
  requestId: string
  deviceId: string
  deviceName: string
  peerIp: string
}

const settings = useAppSettings()
const lan = useLanDiscovery()
const trustRequest = ref<DeviceTrustRequest | null>(null)
const lanConnectRequest = ref<LanConnectRequest | null>(null)
const trustDialogOpen = computed(() => Boolean(trustRequest.value))
const lanConnectDialogOpen = computed(() => Boolean(lanConnectRequest.value))
let unlistenTrust: UnlistenFn | null = null
let unlistenConnection: UnlistenFn | null = null
let unlistenLanDevice: UnlistenFn | null = null
let unlistenLanRequest: UnlistenFn | null = null
let unlistenLanResponse: UnlistenFn | null = null
let pruneTimer: number | null = null

const isTauriRuntime = () =>
  typeof window !== 'undefined' &&
  typeof (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ === 'object'

const prefersReducedMotion = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches

const onRouteBeforeEnter = (element: Element) => {
  if (prefersReducedMotion()) return
  gsap.set(element, { autoAlpha: 0, y: 10 })
}

const onRouteEnter = (element: Element, done: () => void) => {
  if (prefersReducedMotion()) {
    done()
    return
  }
  gsap.to(element, {
    autoAlpha: 1,
    y: 0,
    duration: 0.28,
    ease: 'power2.out',
    clearProps: 'transform,opacity,visibility',
    onComplete: done,
  })
}

const onRouteLeave = (element: Element, done: () => void) => {
  if (prefersReducedMotion()) {
    done()
    return
  }
  gsap.to(element, {
    autoAlpha: 0,
    y: -6,
    duration: 0.16,
    ease: 'power1.in',
    onComplete: done,
  })
}

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

const answerLanConnection = async (decision: 'allowOnce' | 'allowRemember' | 'deny') => {
  const request = lanConnectRequest.value
  if (!request) return

  if (decision === 'deny' || connectionState.value.connected) {
    await invoke('answer_lan_connection', { requestId: request.requestId, accepted: false })
    lanConnectRequest.value = null
    return
  }

  try {
    await invoke('stop_sharing')
    await invoke('start_mouse_client', {
      port: Number(settings.mousePort.value),
      filePort: Number(settings.filePort.value),
      downloadDir: settings.downloadDir.value,
      deviceIdentity: settings.deviceIdentity(),
      trustedDevices: settings.trustedDevices.value,
      clipboardSharingEnabled: settings.clipboardSharingEnabled.value,
    })
    if (decision === 'allowRemember') {
      settings.trustDevice({
        deviceId: request.deviceId,
        deviceName: request.deviceName,
        lastIp: request.peerIp,
      })
    }
    await invoke('answer_lan_connection', { requestId: request.requestId, accepted: true })
  } catch (error: any) {
    lan.connectionMessage.value = `无法接受连接：${error?.message || error || '未知错误'}`
    try {
      await invoke('answer_lan_connection', { requestId: request.requestId, accepted: false })
    } catch {
      // 请求可能已经超时，无需再次处理。
    }
  } finally {
    lanConnectRequest.value = null
  }
}

const handleLanResponse = async (response: LanConnectResponse) => {
  if (lan.pendingRequestId.value !== response.requestId) return
  lan.pendingRequestId.value = null
  lan.connectingDeviceId.value = null

  if (!response.accepted) {
    lan.connectionMessage.value = response.message || `${response.deviceName} 拒绝了连接`
    return
  }

  settings.targetIp.value = response.ip
  settings.mousePort.value = response.mousePort
  lan.connectionMessage.value = `${response.deviceName} 已允许，正在建立连接…`
  try {
    await invoke('stop_sharing')
    await invoke('start_mouse_server', {
      ip: response.ip,
      port: response.mousePort,
      deviceIdentity: settings.deviceIdentity(),
      clipboardSharingEnabled: settings.clipboardSharingEnabled.value,
    })
  } catch (error: any) {
    lan.connectionMessage.value = `连接启动失败：${error?.message || error || '未知错误'}`
  }
}

onMounted(async () => {
  if (!isTauriRuntime()) return

  unlistenTrust = await listen<DeviceTrustRequest>('device-trust-request', (event) => {
    trustRequest.value = event.payload
  })
  unlistenConnection = await listen<ConnectionState>('kms-connection-state', (event) => {
    setConnectionState(event.payload)
  })
  unlistenLanDevice = await listen<LanDevice>('lan-device-upsert', (event) => {
    upsertLanDevice(event.payload)
  })
  unlistenLanRequest = await listen<LanConnectRequest>('lan-connect-request', (event) => {
    lanConnectRequest.value = event.payload
  })
  unlistenLanResponse = await listen<LanConnectResponse>('lan-connect-response', (event) => {
    void handleLanResponse(event.payload)
  })

  try {
    await invoke('start_device_discovery', {
      deviceIdentity: settings.deviceIdentity(),
      mousePort: Number(settings.mousePort.value),
    })
    await invoke('announce_device_now')
  } catch (error: any) {
    lan.connectionMessage.value = `局域网发现启动失败：${error?.message || error || '未知错误'}`
  }
  pruneTimer = window.setInterval(() => pruneLanDevices(), 2_000)

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
  unlistenLanDevice?.()
  unlistenLanRequest?.()
  unlistenLanResponse?.()
  if (pruneTimer !== null) window.clearInterval(pruneTimer)
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
      <div class="flex-1 min-h-0 overflow-hidden">
        <RouterView v-slot="{ Component, route }">
          <Transition
            mode="out-in"
            :css="false"
            @before-enter="onRouteBeforeEnter"
            @enter="onRouteEnter"
            @leave="onRouteLeave"
          >
            <div :key="route.fullPath" class="h-full min-h-0">
              <component :is="Component" />
            </div>
          </Transition>
        </RouterView>
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

  <AlertDialog :open="lanConnectDialogOpen">
    <AlertDialogContent>
      <AlertDialogHeader>
        <AlertDialogTitle>接受局域网连接？</AlertDialogTitle>
        <AlertDialogDescription>
          {{ lanConnectRequest?.deviceName || '未知设备' }} 希望连接并共享键盘与鼠标。
        </AlertDialogDescription>
      </AlertDialogHeader>
      <div class="rounded-md border bg-muted/40 p-3 text-xs text-muted-foreground">
        来源：{{ lanConnectRequest?.peerIp }}<br />
        设备 ID：{{ lanConnectRequest?.deviceId }}
      </div>
      <AlertDialogFooter>
        <AlertDialogCancel @click="answerLanConnection('deny')">拒绝</AlertDialogCancel>
        <Button variant="outline" @click="answerLanConnection('allowOnce')">允许一次</Button>
        <AlertDialogAction @click="answerLanConnection('allowRemember')">允许并记住</AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</template>
