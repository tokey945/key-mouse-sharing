<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useStorage } from '@vueuse/core'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { message } from '@tauri-apps/plugin-dialog'
import { Play, Square } from 'lucide-vue-next'
import RuntimeLogPanel from '@/components/RuntimeLogPanel.vue'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import {
  NumberField,
  NumberFieldContent,
  NumberFieldDecrement,
  NumberFieldIncrement,
  NumberFieldInput,
} from '@/components/ui/number-field'

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

// 基础运行配置会持久化到本地，重启应用后可复用。
const role = useStorage('kms-role', 'server')
const ip = useStorage('kms-ip', '192.168.1.5')
const port = useStorage('kms-port', 4000)
const pairCode = useStorage('kms-pair-code', 'Share2026')

// UI 运行态：isRunning 表示“已发起共享”，isProcessing 表示“命令调用中”。
const isRunning = ref(false)
const isProcessing = ref(false)
const logs = ref<LogItem[]>([])
const logFilter = ref<LogFilter>('all')

const canStart = computed(() => !isRunning.value && !isProcessing.value)
const canStop = computed(() => isRunning.value && !isProcessing.value)
const statusLabel = computed(() => {
  if (isProcessing.value) return '处理中'
  if (isRunning.value) return role.value === 'server' ? '控制端运行中' : '客户端监听中'
  return '空闲'
})
const statusClass = computed(() => (isRunning.value ? 'bg-emerald-500' : isProcessing.value ? 'bg-amber-500' : 'bg-muted-foreground'))

// 统一日志写入入口，限制最大条数避免前端内存持续增长。
const appendLog = (entry: LogItem) => {
  logs.value.push(entry)
  if (logs.value.length > 200) {
    logs.value.splice(0, logs.value.length - 200)
  }
}

const pushLog = (level: LogLevel, msg: string) => {
  appendLog({ at: Date.now(), level, message: msg })
}

const clearLogs = () => {
  logs.value = []
}

const getPortValue = () => Number(port.value)

// 前后端保持一致的参数校验策略，尽量在前端就给出明确提示。
const isValidPort = (value: number) => Number.isInteger(value) && value >= 1 && value <= 65535
const isValidPairCode = (value: string) => value.length >= 8 && /[A-Za-z]/.test(value) && /\d/.test(value)
if (!isValidPairCode(pairCode.value)) {
  pairCode.value = 'Share2026'
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

pushLog('info', '应用就绪，请选择角色后启动共享')

let unlistenLogEvent: UnlistenFn | null = null
onMounted(async () => {
  if (!isTauriRuntime()) {
    pushLog('warn', '当前为浏览器模式，无法订阅后端实时日志事件')
    return
  }

  // 订阅 Rust 侧运行日志（握手成功/失败、限流、连接状态等）。
  try {
    unlistenLogEvent = await listen<RuntimeLogEvent>('kms-log', (event) => {
      const payload = event.payload
      const rawLevel = payload?.level ?? 'info'
      const level: LogLevel = ['info', 'success', 'warn', 'error'].includes(rawLevel)
        ? (rawLevel as LogLevel)
        : 'info'
      const at = payload?.ts_ms ?? Date.now()
      appendLog({ at, level, message: payload?.message ?? '' })
    })
  } catch (error: any) {
    pushLog('warn', `日志事件订阅失败: ${error?.message || '未知错误'}`)
  }
})

onUnmounted(() => {
  // 页面销毁时取消事件订阅，避免重复监听导致日志重复。
  if (unlistenLogEvent) {
    unlistenLogEvent()
    unlistenLogEvent = null
  }
})

const start = async () => {
  // 避免重复触发启动。
  if (!canStart.value) {
    return
  }

  if (!isTauriRuntime()) {
    pushLog('warn', '当前为浏览器模式，无法调用后端共享能力')
    await notify('请使用 `npm run tauri dev` 启动桌面应用后再测试')
    return
  }

  const portValue = getPortValue()
  if (!isValidPort(portValue)) {
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
    // server = 控制端（主动连接目标 IP）; client = 接收端（监听端口）。
    if (role.value === 'server') {
      const targetIp = ip.value.trim()
      if (!targetIp) {
        await notify('请输入目标 IP 地址')
        pushLog('warn', '控制端启动失败：目标 IP 为空')
        return
      }

      await invoke('start_mouse_server', { ip: targetIp, port: portValue, pairCode: currentPairCode })
      isRunning.value = true
      pushLog('success', `控制端已启动，目标 ${targetIp}:${portValue}`)
      await notify('控制端已启动')
      return
    }

    await invoke('start_mouse_client', { port: portValue, pairCode: currentPairCode })
    isRunning.value = true
    pushLog('success', `客户端已启动，监听端口 ${portValue}，等待配对`)
    await notify('客户端已启动')
  } catch (error: any) {
    const msg = error?.message || '未知错误'
    pushLog('error', `启动失败: ${msg}`)
    await notify(`启动失败: ${msg}`)
  } finally {
    isProcessing.value = false
  }
}

const stop = async () => {
  // 避免重复触发停止。
  if (!canStop.value) {
    return
  }

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
        <h1 class="text-base font-semibold">键鼠共享</h1>
        <p class="text-xs text-muted-foreground">触达右侧屏幕边缘后接管远端，远端回到左侧边缘释放。</p>
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

        <div v-if="role === 'server'" class="grid gap-1.5">
          <Label for="target-ip">目标 IP</Label>
          <Input id="target-ip" v-model="ip" type="text" class="focus-visible:ring-0" />
        </div>

        <NumberField id="sharing-port" v-model="port" :format-options="{ useGrouping: false }" :min="1" :max="65535">
          <Label for="sharing-port">端口</Label>
          <NumberFieldContent>
            <NumberFieldDecrement />
            <NumberFieldInput />
            <NumberFieldIncrement />
          </NumberFieldContent>
        </NumberField>

        <div class="grid gap-1.5">
          <Label for="pair-code">配对码</Label>
          <Input id="pair-code" v-model="pairCode" class="focus-visible:ring-0" />
          <p class="text-xs text-muted-foreground">至少 8 位，包含字母和数字。</p>
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
