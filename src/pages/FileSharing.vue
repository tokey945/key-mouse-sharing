<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useStorage } from '@vueuse/core'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { message } from '@tauri-apps/plugin-dialog'
import { FolderOpen, Play, Square, Upload } from 'lucide-vue-next'
import RuntimeLogPanel from '@/components/RuntimeLogPanel.vue'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'

interface LogItem {
  at: number
  level: 'info' | 'success' | 'warn' | 'error'
  message: string
}

interface RuntimeLogEvent {
  level: string
  message: string
  ts_ms?: number
}

const role = useStorage('file-share-role', 'server')
const ip = useStorage('file-share-ip', '192.168.1.5')
const port = useStorage('file-share-port', 5001)
const pairCode = useStorage('file-share-pair-code', 'Share2026')
const downloadDir = useStorage('file-share-download-dir', '')

const selectedFiles = ref<string[]>([])
const isRunning = ref(false)
const isProcessing = ref(false)
const logs = ref<LogItem[]>([])
const logFilter = ref<'all' | 'info' | 'success' | 'warn' | 'error'>('all')
const activeFile = ref('')
const activeProgress = ref(0)

const canStart = computed(() => !isRunning.value && !isProcessing.value)
const canStop = computed(() => isRunning.value && !isProcessing.value)
const statusLabel = computed(() => {
  if (isProcessing.value) return '处理中'
  if (isRunning.value) return role.value === 'server' ? '发送端运行中' : '接收端监听中'
  return '空闲'
})
const statusClass = computed(() => (isRunning.value ? 'bg-emerald-500' : isProcessing.value ? 'bg-amber-500' : 'bg-muted-foreground'))

const appendLog = (entry: LogItem) => {
  logs.value.push(entry)
  if (logs.value.length > 200) {
    logs.value.splice(0, logs.value.length - 200)
  }

  const progress = entry.message.match(/^(.+?) 传输中: ([\d.]+)%/)
  if (progress) {
    activeFile.value = progress[1]
    activeProgress.value = Math.min(100, Number(progress[2]))
  }
  if (entry.message.startsWith('开始接收文件: ') || entry.message.startsWith('开始发送文件: ')) {
    activeFile.value = entry.message.replace(/^开始(接收|发送)文件: /, '').replace(/ \(\d+ bytes\)$/, '')
    activeProgress.value = 0
  }
  if (entry.message.startsWith('文件完成: ') || entry.message.startsWith('文件发送完成: ')) {
    activeProgress.value = 100
  }
}

const pushLog = (level: LogItem['level'], msg: string) => {
  appendLog({ at: Date.now(), level, message: msg })
}

const clearLogs = () => {
  logs.value = []
  activeFile.value = ''
  activeProgress.value = 0
}

const isTauriRuntime = () =>
  typeof window !== 'undefined' &&
  typeof (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ === 'object'
const isValidPairCode = (value: string) => value.length >= 8 && /[A-Za-z]/.test(value) && /\d/.test(value)

if (!isValidPairCode(pairCode.value)) {
  pairCode.value = 'Share2026'
}

const notify = async (text: string) => {
  if (isTauriRuntime()) {
    await message(text)
    return
  }
  pushLog('warn', `[浏览器模式] ${text}`)
}

let unlistenLogEvent: UnlistenFn | null = null
onMounted(() => {
  if (!isTauriRuntime()) {
    pushLog('warn', '当前为浏览器模式，无法订阅后端实时日志事件')
    return
  }

  listen<RuntimeLogEvent>('kms-log', (event) => {
    const payload = event.payload
    const rawLevel = payload?.level ?? 'info'
    const level = ['info', 'success', 'warn', 'error'].includes(rawLevel) ? (rawLevel as LogItem['level']) : 'info'
    const at = payload?.ts_ms ?? Date.now()
    appendLog({ at, level, message: payload?.message ?? '' })
  }).then((fn) => {
    unlistenLogEvent = fn
  }).catch((err) => {
    pushLog('warn', `订阅日志失败: ${err?.toString?.() ?? err}`)
  })
})

onUnmounted(() => {
  if (unlistenLogEvent) {
    unlistenLogEvent()
    unlistenLogEvent = null
  }
})

const chooseFiles = async () => {
  if (!isTauriRuntime()) {
    pushLog('warn', '请在桌面应用模式下使用文件选择功能')
    return
  }

  try {
    const paths = await open({ multiple: true, directory: false })
    if (Array.isArray(paths)) {
      selectedFiles.value = paths.filter(Boolean) as string[]
      pushLog('info', `已选择 ${selectedFiles.value.length} 个文件`)
    }
  } catch (error: any) {
    pushLog('error', `文件选择失败: ${error?.message ?? error}`)
  }
}

const chooseDownloadDir = async () => {
  if (!isTauriRuntime()) {
    pushLog('warn', '请在桌面应用模式下使用目录选择功能')
    return
  }

  try {
    const dir = await open({ directory: true })
    if (typeof dir === 'string' && dir) {
      downloadDir.value = dir
      pushLog('info', `下载目录已设置为 ${dir}`)
    }
  } catch (error: any) {
    pushLog('error', `目录选择失败: ${error?.message ?? error}`)
  }
}

const start = async () => {
  if (!canStart.value) return

  if (!isTauriRuntime()) {
    pushLog('warn', '当前为浏览器模式，无法调用后端共享能力')
    await notify('请使用 `npm run tauri dev` 启动桌面应用后再测试')
    return
  }

  const portValue = Number(port.value)
  if (!Number.isInteger(portValue) || portValue < 1 || portValue > 65535) {
    await notify('端口范围必须是 1-65535')
    pushLog('error', `端口无效: ${portValue}`)
    return
  }

  const currentPairCode = pairCode.value.trim()
  if (!isValidPairCode(currentPairCode)) {
    await notify('配对码至少 8 位，且必须包含字母和数字')
    pushLog('error', '配对码无效：需要至少 8 位并包含字母+数字')
    return
  }

  isProcessing.value = true
  try {
    if (role.value === 'server') {
      if (selectedFiles.value.length === 0) {
        await notify('请先选择要发送的文件')
        pushLog('warn', '未选择文件')
        return
      }

      const targetIp = ip.value.trim()
      if (!targetIp) {
        await notify('请输入目标 IP 地址')
        pushLog('warn', '目标 IP 为空')
        return
      }

      await invoke('start_file_server', {
        ip: targetIp,
        port: portValue,
        pairCode: currentPairCode,
        filePaths: selectedFiles.value,
      })
      isRunning.value = true
      pushLog('success', `文件服务端已启动，目标 ${targetIp}:${portValue}`)
      await notify('文件服务端已启动')
      return
    }

    if (!downloadDir.value) {
      await notify('请先选择下载目录')
      pushLog('warn', '下载目录未设置')
      return
    }

    await invoke('start_file_client', {
      port: portValue,
      pairCode: currentPairCode,
      downloadDir: downloadDir.value,
    })
    isRunning.value = true
    pushLog('success', `文件客户端已启动，监听端口 ${portValue}`)
    await notify('文件客户端已启动')
  } catch (error: any) {
    const msg = error?.message || '未知错误'
    pushLog('error', `启动失败: ${msg}`)
    await notify(`启动失败: ${msg}`)
  } finally {
    isProcessing.value = false
  }
}

const stop = async () => {
  if (!canStop.value) return

  if (!isTauriRuntime()) {
    pushLog('warn', '当前为浏览器模式，无共享会话可停止')
    return
  }

  isProcessing.value = true
  try {
    await invoke('stop_sharing')
    isRunning.value = false
    pushLog('info', '已停止共享')
    await notify('已停止共享')
  } catch (error: any) {
    const msg = error?.message || '未知错误'
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
        <h1 class="text-base font-semibold">文件共享</h1>
        <p class="text-xs text-muted-foreground">局域网直连传输，完成时校验大小与传输校验值。</p>
      </div>
      <div class="flex items-center gap-2 rounded-md border px-3 py-1.5 text-sm">
        <span class="size-2 rounded-full" :class="statusClass" />
        {{ statusLabel }}
      </div>
    </header>

    <div class="grid min-h-0 flex-1 gap-4 lg:grid-cols-[380px_minmax(0,1fr)]">
      <section class="space-y-4 rounded-lg border bg-background p-4">
        <div class="space-y-2">
          <Label>角色</Label>
          <RadioGroup v-model="role" default-value="server" :orientation="'horizontal'" class="grid grid-cols-2 gap-2">
            <Label for="fs-r1" class="flex cursor-pointer items-center gap-2 rounded-md border px-3 py-2 text-sm">
              <RadioGroupItem id="fs-r1" value="server" />
              发送端
            </Label>
            <Label for="fs-r2" class="flex cursor-pointer items-center gap-2 rounded-md border px-3 py-2 text-sm">
              <RadioGroupItem id="fs-r2" value="client" />
              接收端
            </Label>
          </RadioGroup>
        </div>

        <div v-if="role === 'server'" class="grid gap-1.5">
          <Label for="target-ip">目标 IP</Label>
          <Input id="target-ip" v-model="ip" type="text" class="focus-visible:ring-0" />
        </div>

        <div class="grid gap-1.5">
          <Label for="sharing-port">端口</Label>
          <Input id="sharing-port" v-model.number="port" type="number" min="1" max="65535" class="focus-visible:ring-0" />
        </div>

        <div class="grid gap-1.5">
          <Label for="pair-code">配对码</Label>
          <Input id="pair-code" v-model="pairCode" class="focus-visible:ring-0" />
          <p class="text-xs text-muted-foreground">至少 8 位，包含字母和数字。</p>
        </div>

        <div v-if="role === 'server'" class="space-y-2 rounded-md border p-3">
          <Button type="button" size="sm" variant="outline" class="gap-2" @click="chooseFiles">
            <Upload class="size-4" />
            选择文件
          </Button>
          <div class="text-xs text-muted-foreground">已选 {{ selectedFiles.length }} 个文件</div>
          <ul class="max-h-28 space-y-1 overflow-hidden text-xs">
            <li v-for="file in selectedFiles" :key="file" class="truncate rounded bg-muted px-2 py-1">{{ file }}</li>
          </ul>
        </div>

        <div v-if="role === 'client'" class="space-y-2 rounded-md border p-3">
          <Button type="button" size="sm" variant="outline" class="gap-2" @click="chooseDownloadDir">
            <FolderOpen class="size-4" />
            选择下载目录
          </Button>
          <div class="truncate text-xs text-muted-foreground">下载目录：{{ downloadDir || '未设置' }}</div>
        </div>

        <div class="rounded-md border p-3">
          <div class="mb-2 flex items-center justify-between text-xs">
            <span class="truncate text-muted-foreground">{{ activeFile || '暂无活动文件' }}</span>
            <span>{{ activeProgress.toFixed(0) }}%</span>
          </div>
          <div class="h-2 overflow-hidden rounded-full bg-muted">
            <div class="h-full bg-primary transition-all" :style="{ width: `${activeProgress}%` }" />
          </div>
        </div>

        <div class="flex gap-2 pt-2">
          <Button class="flex-1 gap-2" @click="start" :disabled="!canStart">
            <Play class="size-4" />
            启动
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
