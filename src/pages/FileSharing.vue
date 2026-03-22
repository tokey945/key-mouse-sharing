<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useStorage } from '@vueuse/core'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { message } from '@tauri-apps/plugin-dialog'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
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
const pairCode = useStorage('file-share-pair-code', '1234567')
const downloadDir = useStorage('file-share-download-dir', '')

const selectedFiles = ref<string[]>([])
const isRunning = ref(false)
const isProcessing = ref(false)
const logs = ref<LogItem[]>([])
const logFilter = ref<'all' | 'info' | 'success' | 'warn' | 'error'>('all')

const canStart = computed(() => !isRunning.value && !isProcessing.value)
const canStop = computed(() => isRunning.value && !isProcessing.value)

const appendLog = (entry: LogItem) => {
  logs.value.push(entry)
  if (logs.value.length > 200) {
    logs.value.splice(0, logs.value.length - 200)
  }
}

const pushLog = (level: LogItem['level'], msg: string) => {
  appendLog({ at: Date.now(), level, message: msg })
}

const logStats = computed(() => {
  const stats = { all: logs.value.length, info: 0, success: 0, warn: 0, error: 0 }
  for (const item of logs.value) {
    stats[item.level] += 1
  }
  return stats
})

const filteredLogs = computed(() => {
  if (logFilter.value === 'all') return logs.value
  return logs.value.filter((item) => item.level === logFilter.value)
})

const clearLogs = () => { logs.value = [] }

const getLogClass = (level: LogItem['level']) => {
  switch (level) {
    case 'error':
      return 'text-red-500'
    case 'warn':
      return 'text-yellow-500'
    case 'success':
      return 'text-green-500'
    default:
      return ''
  }
}

const isTauriRuntime = () =>
  typeof window !== 'undefined' &&
  typeof (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ === 'object'

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
  if (currentPairCode.length < 8 || !/[A-Za-z]/.test(currentPairCode) || !/\d/.test(currentPairCode)) {
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
  <div class="p-4 space-y-4">
    <Label>角色：</Label>
    <RadioGroup v-model="role" default-value="server" :orientation="'horizontal'" class="flex space-x-4">
      <div class="flex items-center space-x-2">
        <RadioGroupItem id="fs-r1" value="server" />
        <Label for="fs-r1">服务端（发送文件）</Label>
      </div>
      <div class="flex items-center space-x-2">
        <RadioGroupItem id="fs-r2" value="client" />
        <Label for="fs-r2">客户端（接收文件）</Label>
      </div>
    </RadioGroup>

    <div class="grid w-full max-w-sm items-center gap-1.5">
      <Label for="target-ip">目标 IP：</Label>
      <Input id="target-ip" v-model="ip" type="text" class="w-56 focus-visible:ring-0" />
    </div>

    <div class="grid w-full max-w-sm items-center gap-1.5">
      <Label for="sharing-port">端口：</Label>
      <Input id="sharing-port" v-model.number="port" type="number" min="1" max="65535"
        class="w-56 focus-visible:ring-0" />
    </div>

    <div class="grid w-full max-w-sm items-center gap-1.5">
      <Label for="pair-code">配对码：</Label>
      <Input id="pair-code" v-model="pairCode" class="w-56 focus-visible:ring-0" />
    </div>

    <div v-if="role === 'server'" class="space-y-2">
      <Button type="button" size="sm" @click="chooseFiles">选择文件</Button>
      <div class="text-sm text-muted-foreground">
        已选文件：{{ selectedFiles.length }} 个
        <ul class="list-disc pl-5">
          <li v-for="file in selectedFiles" :key="file" class="truncate">{{ file }}</li>
        </ul>
      </div>
    </div>

    <div v-if="role === 'client'" class="space-y-2">
      <Button type="button" size="sm" @click="chooseDownloadDir">选择下载目录</Button>
      <div class="text-sm text-muted-foreground">下载目录：{{ downloadDir || '未设置' }}</div>
    </div>

    <div class="space-x-4">
      <Button @click="start" :disabled="!canStart">启动共享</Button>
      <Button @click="stop" variant="destructive" :disabled="!canStop">停止共享</Button>
    </div>

    <div class="w-full max-w-2xl h-56 p-0 m-auto">
      <div class="mb-2 flex flex-wrap items-center gap-2">
        <h6 class="text-lg font-semibold mr-2">日志</h6>
        <Button size="sm" :variant="logFilter === 'all' ? 'default' : 'outline'" @click="logFilter = 'all'">全部 {{
          logStats.all }}</Button>
        <Button size="sm" :variant="logFilter === 'info' ? 'default' : 'outline'" @click="logFilter = 'info'">INFO {{
          logStats.info }}</Button>
        <Button size="sm" :variant="logFilter === 'success' ? 'default' : 'outline'"
          @click="logFilter = 'success'">SUCCESS {{ logStats.success }}</Button>
        <Button size="sm" :variant="logFilter === 'warn' ? 'default' : 'outline'" @click="logFilter = 'warn'">WARN {{
          logStats.warn }}</Button>
        <Button size="sm" :variant="logFilter === 'error' ? 'default' : 'outline'" @click="logFilter = 'error'">ERROR {{
          logStats.error }}</Button>
        <Button size="sm" variant="ghost" class="ml-auto" @click="clearLogs">清空</Button>
      </div>

      <ScrollArea class="w-full h-full rounded-md border p-4 bg-accent/30">
        <div class="font-mono text-sm space-y-1">
          <div v-for="(log, index) in filteredLogs" :key="`${log.at}-${index}`" :class="getLogClass(log.level)">
            {{ new Date(log.at).toLocaleTimeString() }} [{{ log.level.toUpperCase() }}]: {{ log.message }}
          </div>
          <div v-if="filteredLogs.length === 0" class="text-muted-foreground">暂无日志</div>
        </div>
      </ScrollArea>
    </div>
  </div>
</template>
