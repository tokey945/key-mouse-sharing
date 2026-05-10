<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Activity, EthernetPort, Network, Radar } from 'lucide-vue-next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  NumberField,
  NumberFieldContent,
  NumberFieldDecrement,
  NumberFieldIncrement,
  NumberFieldInput,
} from '@/components/ui/number-field'
import { useAppSettings } from '@/composables/useAppSettings'

interface NetworkSummary {
  primaryIp?: string | null
  loopbackIp: string
  hostname: string
}

interface PortCheckResult {
  port: number
  available: boolean
  message: string
}

interface TcpProbeResult {
  reachable: boolean
  elapsedMs: number
  message: string
}

const settings = useAppSettings()
const summary = ref<NetworkSummary | null>(null)
const portToCheck = ref(Number(settings.mousePort.value))
const probeIp = ref(settings.targetIp.value)
const probePort = ref(Number(settings.mousePort.value))
const probeTimeout = ref(1200)
const portResult = ref<PortCheckResult | null>(null)
const probeResult = ref<TcpProbeResult | null>(null)
const loadingSummary = ref(false)
const checkingPort = ref(false)
const probing = ref(false)
const browserNotice = ref('')

const isTauriRuntime = () =>
  typeof window !== 'undefined' && typeof (window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ === 'object'

const canRunNative = computed(() => isTauriRuntime())

const refreshSummary = async () => {
  if (!canRunNative.value) {
    browserNotice.value = '浏览器预览无法读取本机网络摘要，请在 Tauri 桌面应用中使用。'
    return
  }
  loadingSummary.value = true
  try {
    summary.value = await invoke<NetworkSummary>('get_network_summary')
  } finally {
    loadingSummary.value = false
  }
}

const checkPort = async () => {
  if (!canRunNative.value) {
    browserNotice.value = '浏览器预览无法检查端口占用，请在 Tauri 桌面应用中使用。'
    return
  }
  checkingPort.value = true
  try {
    portResult.value = await invoke<PortCheckResult>('check_port_available', {
      port: Number(portToCheck.value),
    })
  } finally {
    checkingPort.value = false
  }
}

const probeTcp = async () => {
  if (!canRunNative.value) {
    browserNotice.value = '浏览器预览无法发起 TCP 探测，请在 Tauri 桌面应用中使用。'
    return
  }
  probing.value = true
  try {
    probeResult.value = await invoke<TcpProbeResult>('probe_tcp_connection', {
      ip: probeIp.value,
      port: Number(probePort.value),
      timeoutMs: Number(probeTimeout.value),
    })
  } finally {
    probing.value = false
  }
}

onMounted(refreshSummary)
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-4 p-4 pr-6">
    <header class="flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-background px-4 py-3">
      <div>
        <h1 class="text-base font-semibold">开发工具</h1>
        <p class="text-xs text-muted-foreground">用于程序员调试局域网连接、端口占用和远端可达性。</p>
      </div>
      <div class="flex items-center gap-2 rounded-md border px-3 py-1.5 text-sm">
        <span class="size-2 rounded-full" :class="canRunNative ? 'bg-emerald-500' : 'bg-amber-500'" />
        {{ canRunNative ? '桌面运行时' : '浏览器预览' }}
      </div>
    </header>

    <div v-if="browserNotice" class="rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900 dark:border-amber-900/50 dark:bg-amber-950/30 dark:text-amber-200">
      {{ browserNotice }}
    </div>

    <div class="grid min-h-0 flex-1 gap-4 xl:grid-cols-[360px_360px_minmax(0,1fr)]">
      <section class="space-y-4 rounded-lg border bg-background p-4">
        <div class="flex items-center gap-2">
          <Network class="size-4 text-primary" />
          <h2 class="text-sm font-semibold">本机网络摘要</h2>
        </div>

        <div class="space-y-2 rounded-md border p-3 text-sm">
          <div class="flex items-center justify-between">
            <span class="text-muted-foreground">主机名</span>
            <span class="font-medium">{{ summary?.hostname || '未知' }}</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-muted-foreground">局域网 IP</span>
            <span class="font-medium">{{ summary?.primaryIp || '未检测到' }}</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-muted-foreground">Loopback</span>
            <span class="font-medium">{{ summary?.loopbackIp || '127.0.0.1' }}</span>
          </div>
        </div>

        <Button variant="outline" class="w-full gap-2" @click="refreshSummary" :disabled="loadingSummary">
          <Radar class="size-4" />
          {{ loadingSummary ? '刷新中' : '刷新摘要' }}
        </Button>
      </section>

      <section class="space-y-4 rounded-lg border bg-background p-4">
        <div class="flex items-center gap-2">
          <EthernetPort class="size-4 text-primary" />
          <h2 class="text-sm font-semibold">端口占用检查</h2>
        </div>

        <NumberField v-model="portToCheck" :format-options="{ useGrouping: false }" :min="1" :max="65535">
          <Label>监听端口</Label>
          <NumberFieldContent>
            <NumberFieldDecrement />
            <NumberFieldInput />
            <NumberFieldIncrement />
          </NumberFieldContent>
        </NumberField>

        <Button class="w-full" @click="checkPort" :disabled="checkingPort">
          {{ checkingPort ? '检查中' : '检查端口' }}
        </Button>

        <div v-if="portResult" class="rounded-md border p-3 text-sm" :class="portResult.available ? 'border-emerald-300 bg-emerald-50 text-emerald-900 dark:border-emerald-900/60 dark:bg-emerald-950/30 dark:text-emerald-200' : 'border-red-300 bg-red-50 text-red-900 dark:border-red-900/60 dark:bg-red-950/30 dark:text-red-200'">
          {{ portResult.message }}
        </div>
      </section>

      <section class="space-y-4 rounded-lg border bg-background p-4">
        <div class="flex items-center gap-2">
          <Activity class="size-4 text-primary" />
          <h2 class="text-sm font-semibold">TCP 连接探测</h2>
        </div>

        <div class="grid gap-3 md:grid-cols-[1fr_160px_160px]">
          <div class="grid gap-1.5">
            <Label for="probe-ip">目标 IP</Label>
            <Input id="probe-ip" v-model="probeIp" class="focus-visible:ring-0" />
          </div>
          <NumberField v-model="probePort" :format-options="{ useGrouping: false }" :min="1" :max="65535">
            <Label>端口</Label>
            <NumberFieldContent>
              <NumberFieldDecrement />
              <NumberFieldInput />
              <NumberFieldIncrement />
            </NumberFieldContent>
          </NumberField>
          <NumberField v-model="probeTimeout" :format-options="{ useGrouping: false }" :min="200" :max="10000">
            <Label>超时 ms</Label>
            <NumberFieldContent>
              <NumberFieldDecrement />
              <NumberFieldInput />
              <NumberFieldIncrement />
            </NumberFieldContent>
          </NumberField>
        </div>

        <Button class="w-full" @click="probeTcp" :disabled="probing">
          {{ probing ? '探测中' : '探测连接' }}
        </Button>

        <div v-if="probeResult" class="rounded-md border p-3 text-sm" :class="probeResult.reachable ? 'border-emerald-300 bg-emerald-50 text-emerald-900 dark:border-emerald-900/60 dark:bg-emerald-950/30 dark:text-emerald-200' : 'border-amber-300 bg-amber-50 text-amber-900 dark:border-amber-900/60 dark:bg-amber-950/30 dark:text-amber-200'">
          {{ probeResult.message }} · {{ probeResult.elapsedMs }}ms
        </div>

        <div class="rounded-md border border-dashed p-3 text-xs leading-5 text-muted-foreground">
          后续适合继续加入：UDP 设备发现、剪贴板同步测试、JSON/Base64 小工具、局域网延迟采样、文件传输压力测试。
        </div>
      </section>
    </div>
  </div>
</template>
