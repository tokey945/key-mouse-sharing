<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { open, message } from '@tauri-apps/plugin-dialog'
import { Send, Settings, Trash2, Upload, X } from 'lucide-vue-next'
import RuntimeLogPanel from '@/components/RuntimeLogPanel.vue'
import { Button } from '@/components/ui/button'
import { useAppSettings } from '@/composables/useAppSettings'
import { connectionState } from '@/composables/useConnectionState'

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

const router = useRouter()
const settings = useAppSettings()
const selectedFiles = ref<string[]>([])
const isProcessing = ref(false)
const logs = ref<LogItem[]>([])
const logFilter = ref<'all' | 'info' | 'success' | 'warn' | 'error'>('all')
const activeFile = ref('')
const activeProgress = ref(0)
const isDraggingFiles = ref(false)

const statusLabel = computed(() => {
  if (connectionState.value.connected) return `已连接 ${connectionState.value.peerDeviceName || connectionState.value.peerIp || '远端'}`
  return '等待键鼠共享连接'
})
const statusClass = computed(() => connectionState.value.connected ? 'bg-emerald-500' : 'bg-amber-500')
const canSend = computed(() => connectionState.value.connected && selectedFiles.value.length > 0 && !isProcessing.value)

const appendLog = (entry: LogItem) => {
  logs.value.push(entry)
  if (logs.value.length > 200) logs.value.splice(0, logs.value.length - 200)

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

const pushLog = (level: LogItem['level'], msg: string) => appendLog({ at: Date.now(), level, message: msg })
const clearLogs = () => {
  logs.value = []
  activeFile.value = ''
  activeProgress.value = 0
}

const addFiles = (paths: string[]) => {
  const normalized = paths.filter(Boolean)
  if (normalized.length === 0) return

  const seen = new Set(selectedFiles.value)
  let added = 0
  for (const path of normalized) {
    if (seen.has(path)) continue
    selectedFiles.value.push(path)
    seen.add(path)
    added += 1
  }

  pushLog('info', added > 0 ? `已加入 ${added} 个文件` : '拖入的文件已在列表中')
}

const removeFile = (file: string) => {
  selectedFiles.value = selectedFiles.value.filter((item) => item !== file)
}

const clearSelectedFiles = () => {
  selectedFiles.value = []
  pushLog('info', '已清空待发送文件')
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

let unlistenLogEvent: UnlistenFn | null = null
let unlistenDragDropEvent: UnlistenFn | null = null
onMounted(() => {
  if (!connectionState.value.connected) {
    pushLog('warn', '文件共享需要先建立键鼠共享连接')
  }

  if (!isTauriRuntime()) {
    pushLog('warn', '当前为浏览器模式，无法订阅后端实时日志事件')
    pushLog('warn', '当前为浏览器模式，文件拖拽仅在桌面应用内可用')
    return
  }

  listen<RuntimeLogEvent>('kms-log', (event) => {
    const payload = event.payload
    const rawLevel = payload?.level ?? 'info'
    const level = ['info', 'success', 'warn', 'error'].includes(rawLevel) ? (rawLevel as LogItem['level']) : 'info'
    appendLog({ at: payload?.ts_ms ?? Date.now(), level, message: payload?.message ?? '' })
  }).then((fn) => {
    unlistenLogEvent = fn
  }).catch((err) => {
    pushLog('warn', `订阅日志失败: ${err?.toString?.() ?? err}`)
  })

  getCurrentWebview().onDragDropEvent((event) => {
    const payload = event.payload
    if (payload.type === 'enter' || payload.type === 'over') {
      isDraggingFiles.value = true
      return
    }
    if (payload.type === 'leave') {
      isDraggingFiles.value = false
      return
    }
    if (payload.type === 'drop') {
      isDraggingFiles.value = false
      addFiles(payload.paths)
      if (!connectionState.value.connected) {
        pushLog('warn', '文件已加入列表，请先建立键鼠共享连接后再发送')
      }
    }
  }).then((fn) => {
    unlistenDragDropEvent = fn
  }).catch((err) => {
    pushLog('warn', `文件拖拽监听失败: ${err?.toString?.() ?? err}`)
  })
})

onUnmounted(() => {
  unlistenLogEvent?.()
  unlistenDragDropEvent?.()
})

const chooseFiles = async () => {
  if (!isTauriRuntime()) {
    pushLog('warn', '请在桌面应用模式下使用文件选择功能')
    return
  }

  try {
    const paths = await open({ multiple: true, directory: false })
    if (Array.isArray(paths)) {
      addFiles(paths.filter(Boolean) as string[])
    }
  } catch (error: any) {
    pushLog('error', `文件选择失败: ${error?.message ?? error}`)
  }
}

const goConnect = async () => {
  await notify('请先建立键鼠共享连接')
  router.push('/')
}

const send = async () => {
  if (!connectionState.value.connected || !connectionState.value.peerIp) {
    await goConnect()
    return
  }
  if (selectedFiles.value.length === 0) {
    await notify('请先选择要发送的文件')
    return
  }

  isProcessing.value = true
  try {
    await invoke('send_files', {
      ip: connectionState.value.peerIp,
      port: Number(settings.filePort.value),
      deviceIdentity: settings.deviceIdentity(),
      filePaths: selectedFiles.value,
    })
    pushLog('success', `已发起 ${selectedFiles.value.length} 个文件的发送任务`)
  } catch (error: any) {
    const msg = error?.message || error || '未知错误'
    pushLog('error', `发送失败: ${msg}`)
    await notify(`发送失败: ${msg}`)
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
        <p class="text-xs text-muted-foreground">复用已建立的键鼠信任关系，通过独立文件端口传输并校验完整性。</p>
      </div>
      <div class="flex items-center gap-2 rounded-md border px-3 py-1.5 text-sm">
        <span class="size-2 rounded-full" :class="statusClass" />
        {{ statusLabel }}
      </div>
    </header>

    <div class="grid min-h-0 flex-1 gap-4 lg:grid-cols-[380px_minmax(0,1fr)]">
      <section class="space-y-4 rounded-lg border bg-background p-4">
        <div v-if="!connectionState.connected" class="rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-900 dark:border-amber-900/50 dark:bg-amber-950/30 dark:text-amber-200">
          还没有键鼠共享连接。请先连接到另一台设备，文件共享会自动复用该连接。
        </div>

        <div
          class="space-y-3 rounded-md border border-dashed p-3 transition-colors"
          :class="isDraggingFiles ? 'border-primary bg-primary/5' : 'border-border bg-background'"
        >
          <div class="flex items-center justify-between gap-2">
            <Button type="button" size="sm" variant="outline" class="gap-2" @click="chooseFiles">
              <Upload class="size-4" />
              选择文件
            </Button>
            <Button
              v-if="selectedFiles.length > 0"
              type="button"
              size="sm"
              variant="ghost"
              class="gap-2"
              @click="clearSelectedFiles"
            >
              <Trash2 class="size-4" />
              清空
            </Button>
          </div>
          <div class="rounded-md bg-muted/60 px-3 py-2 text-xs text-muted-foreground">
            {{ isDraggingFiles ? '松开鼠标加入待发送列表' : '可将文件拖到这里，拖入后不会自动发送' }}
          </div>
          <div class="text-xs text-muted-foreground">待发送 {{ selectedFiles.length }} 个文件</div>
          <ul class="max-h-44 space-y-1 overflow-auto pr-1 text-xs">
            <li
              v-for="file in selectedFiles"
              :key="file"
              class="flex items-center gap-2 rounded bg-muted px-2 py-1"
            >
              <span class="min-w-0 flex-1 truncate">{{ file }}</span>
              <Button type="button" size="icon" variant="ghost" class="size-6 shrink-0" @click="removeFile(file)">
                <X class="size-3.5" />
              </Button>
            </li>
          </ul>
        </div>

        <div class="space-y-2 rounded-md border p-3 text-sm">
          <div class="flex items-center justify-between">
            <span class="text-muted-foreground">目标</span>
            <span class="font-medium">{{ connectionState.peerIp || '未连接' }}</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-muted-foreground">文件端口</span>
            <span class="font-medium">{{ settings.filePort.value }}</span>
          </div>
          <RouterLink to="/settings" class="inline-flex items-center gap-2 text-xs text-primary hover:underline">
            <Settings class="size-3.5" />
            修改文件端口或接收目录
          </RouterLink>
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
          <Button v-if="connectionState.connected" class="flex-1 gap-2" @click="send" :disabled="!canSend">
            <Send class="size-4" />
            发送文件
          </Button>
          <Button v-else class="flex-1 gap-2" variant="secondary" @click="goConnect">
            先连接键鼠共享
          </Button>
        </div>
      </section>

      <RuntimeLogPanel :logs="logs" v-model:filter="logFilter" @clear="clearLogs" />
    </div>
  </div>
</template>
