<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { message } from '@tauri-apps/plugin-dialog'
import { Play, Settings, Square } from 'lucide-vue-next'
import { RouterLink } from 'vue-router'
import RuntimeLogPanel from '@/components/RuntimeLogPanel.vue'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { useAppSettings } from '@/composables/useAppSettings'
import { connectionState } from '@/composables/useConnectionState'

type LogLevel = 'info' | 'success' | 'warn' | 'error'
type LogFilter = 'all' | LogLevel

interface LogItem {
  at: number
  level: LogLevel
  message: string
}

interface RuntimeLogEvent {
  level: string
  message: string
  ts_ms?: number
}

const settings = useAppSettings()
const role = settings.defaultRole
const isRunning = ref(false)
const isProcessing = ref(false)
const logs = ref<LogItem[]>([])
const logFilter = ref<LogFilter>('all')

const canStart = computed(() => !isRunning.value && !isProcessing.value)
const canStop = computed(() => isRunning.value && !isProcessing.value)
const statusLabel = computed(() => {
  if (isProcessing.value) return '处理中'
  if (connectionState.value.connected) return `已连接 ${connectionState.value.peerDeviceName || connectionState.value.peerIp || '远端'}`
  if (isRunning.value) return role.value === 'server' ? '控制端连接中' : '客户端监听中'
  return '空闲'
})
const statusClass = computed(() => {
  if (connectionState.value.connected) return 'bg-emerald-500'
  if (isRunning.value || isProcessing.value) return 'bg-amber-500'
  return 'bg-muted-foreground'
})

const appendLog = (entry: LogItem) => {
  logs.value.push(entry)
  if (logs.value.length > 200) logs.value.splice(0, logs.value.length - 200)
}
const pushLog = (level: LogLevel, msg: string) => appendLog({ at: Date.now(), level, message: msg })
const clearLogs = () => {
  logs.value = []
}

const isTauriRuntime = () =>
  typeof window !== 'undefined' && typeof (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ === 'object'

const notify = async (text: string) => {
  if (isTauriRuntime()) {
    await message(text)
    return
  }
  pushLog('warn', `[浏览器模式] ${text}`)
}

const isValidPort = (value: number) => Number.isInteger(Number(value)) && Number(value) >= 1 && Number(value) <= 65535

let unlistenLogEvent: UnlistenFn | null = null
onMounted(async () => {
  pushLog('info', '应用就绪。端口、目标 IP 和下载目录可在设置页调整。')
  if (!isTauriRuntime()) {
    pushLog('warn', '当前为浏览器模式，无法订阅后端实时日志事件')
    return
  }

  try {
    unlistenLogEvent = await listen<RuntimeLogEvent>('kms-log', (event) => {
      const payload = event.payload
      const rawLevel = payload?.level ?? 'info'
      const level: LogLevel = ['info', 'success', 'warn', 'error'].includes(rawLevel) ? (rawLevel as LogLevel) : 'info'
      appendLog({ at: payload?.ts_ms ?? Date.now(), level, message: payload?.message ?? '' })
    })
  } catch (error: any) {
    pushLog('warn', `日志事件订阅失败: ${error?.message || '未知错误'}`)
  }
})

onUnmounted(() => {
  unlistenLogEvent?.()
})

const start = async () => {
  if (!canStart.value) return
  if (!isTauriRuntime()) {
    await notify('请使用 `npm run tauri dev` 启动桌面应用后再测试')
    return
  }

  if (!isValidPort(Number(settings.mousePort.value)) || !isValidPort(Number(settings.filePort.value))) {
    await notify('端口范围必须是 1-65535，请到设置页调整')
    pushLog('error', '端口无效，请检查设置')
    return
  }

  isProcessing.value = true
  try {
    if (role.value === 'server') {
      const targetIp = settings.targetIp.value.trim()
      if (!targetIp) {
        await notify('请先在设置页填写默认目标 IP')
        return
      }
      await invoke('start_mouse_server', {
        ip: targetIp,
        port: Number(settings.mousePort.value),
        deviceIdentity: settings.deviceIdentity(),
        clipboardSharingEnabled: settings.clipboardSharingEnabled.value,
      })
      pushLog('success', `控制端已启动，目标 ${targetIp}:${settings.mousePort.value}`)
    } else {
      if (!settings.downloadDir.value) {
        await notify('请先在设置页选择文件接收目录')
        return
      }
      await invoke('start_mouse_client', {
        port: Number(settings.mousePort.value),
        filePort: Number(settings.filePort.value),
        downloadDir: settings.downloadDir.value,
        deviceIdentity: settings.deviceIdentity(),
        trustedDevices: settings.trustedDevices.value,
        clipboardSharingEnabled: settings.clipboardSharingEnabled.value,
      })
      pushLog('success', `客户端已启动，键鼠端口 ${settings.mousePort.value}，文件端口 ${settings.filePort.value}`)
    }
    isRunning.value = true
  } catch (error: any) {
    const msg = error?.message || error || '未知错误'
    pushLog('error', `启动失败: ${msg}`)
    await notify(`启动失败: ${msg}`)
  } finally {
    isProcessing.value = false
  }
}

const stop = async () => {
  if (!canStop.value) return
  if (!isTauriRuntime()) return

  isProcessing.value = true
  try {
    await invoke('stop_sharing')
    isRunning.value = false
    pushLog('info', '已停止共享')
  } catch (error: any) {
    const msg = error?.message || error || '未知错误'
    pushLog('error', `停止失败: ${msg}`)
    await notify(`停止失败: ${msg}`)
  } finally {
    isProcessing.value = false
  }
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-4 p-4 pr-6">
    <header class="flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-background px-4 py-3">
      <div>
        <h1 class="text-base font-semibold">键鼠共享</h1>
        <p class="text-xs text-muted-foreground">触达右侧屏幕边缘后接管远端，文件接收会随客户端监听自动开启。</p>
      </div>
      <div class="flex items-center gap-2 rounded-md border px-3 py-1.5 text-sm">
        <span class="size-2 rounded-full" :class="statusClass" />
        {{ statusLabel }}
      </div>
    </header>

    <div class="grid min-h-0 flex-1 gap-4 lg:grid-cols-[360px_minmax(0,1fr)]">
      <section class="space-y-4 rounded-lg border bg-background p-4">
        <div class="space-y-2">
          <Label>本机角色</Label>
          <RadioGroup v-model="role" default-value="server" :orientation="'horizontal'" class="grid grid-cols-2 gap-2">
            <Label for="r1" class="flex cursor-pointer items-center gap-2 rounded-md border px-3 py-2 text-sm">
              <RadioGroupItem id="r1" value="server" />
              控制端
            </Label>
            <Label for="r2" class="flex cursor-pointer items-center gap-2 rounded-md border px-3 py-2 text-sm">
              <RadioGroupItem id="r2" value="client" />
              客户端
            </Label>
          </RadioGroup>
        </div>

        <div class="space-y-2 rounded-md border p-3 text-sm">
          <div class="flex items-center justify-between">
            <span class="text-muted-foreground">本机名称</span>
            <span class="font-medium">{{ settings.deviceName.value }}</span>
          </div>
          <div v-if="role === 'server'" class="flex items-center justify-between">
            <span class="text-muted-foreground">目标 IP</span>
            <span class="font-medium">{{ settings.targetIp.value }}</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-muted-foreground">键鼠端口</span>
            <span class="font-medium">{{ settings.mousePort.value }}</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-muted-foreground">剪贴板同步</span>
            <span class="font-medium">{{ settings.clipboardSharingEnabled.value ? '开启' : '关闭' }}</span>
          </div>
          <div v-if="role === 'client'" class="flex items-center justify-between">
            <span class="text-muted-foreground">文件端口</span>
            <span class="font-medium">{{ settings.filePort.value }}</span>
          </div>
          <RouterLink to="/settings" class="inline-flex items-center gap-2 text-xs text-primary hover:underline">
            <Settings class="size-3.5" />
            修改默认连接设置
          </RouterLink>
        </div>

        <div class="flex gap-2 pt-2">
          <Button class="flex-1 gap-2" @click="start" :disabled="!canStart">
            <Play class="size-4" />
            启动共享
          </Button>
          <Button class="flex-1 gap-2" @click="stop" variant="destructive" :disabled="!canStop">
            <Square class="size-4" />
            停止
          </Button>
        </div>
      </section>

      <RuntimeLogPanel :logs="logs" v-model:filter="logFilter" @clear="clearLogs" />
    </div>
  </div>
</template>
