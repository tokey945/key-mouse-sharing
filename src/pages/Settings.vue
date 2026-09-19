<script setup lang="ts">
import { open } from '@tauri-apps/plugin-dialog'
import { Clipboard, FolderOpen, Radar, Trash2 } from 'lucide-vue-next'
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

const settings = useAppSettings()

const chooseDownloadDir = async () => {
  const dir = await open({ directory: true })
  if (typeof dir === 'string' && dir) {
    settings.downloadDir.value = dir
  }
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-4 p-4 pr-6">
    <header class="flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-background px-4 py-3">
      <div>
        <h1 class="text-base font-semibold">设置</h1>
        <p class="text-xs text-muted-foreground">集中管理连接默认值、文件接收目录和受信设备。</p>
      </div>
      <div class="rounded-md border px-3 py-1.5 text-xs text-muted-foreground">
        本机 ID：{{ settings.deviceId.value.slice(0, 8) }}
      </div>
    </header>

    <div class="grid min-h-0 flex-1 gap-4 xl:grid-cols-[420px_minmax(0,1fr)]">
      <section class="space-y-4 rounded-lg border bg-background p-4">
        <div class="grid gap-1.5">
          <Label for="device-name">设备名称</Label>
          <Input id="device-name" v-model="settings.deviceName.value" class="focus-visible:ring-0" />
        </div>

        <div class="grid gap-1.5">
          <Label for="target-ip">默认目标 IP</Label>
          <Input id="target-ip" v-model="settings.targetIp.value" class="focus-visible:ring-0" />
        </div>

        <div class="grid grid-cols-2 gap-3">
          <NumberField v-model="settings.mousePort.value" :format-options="{ useGrouping: false }" :min="1" :max="65535">
            <Label>键鼠端口</Label>
            <NumberFieldContent>
              <NumberFieldDecrement />
              <NumberFieldInput />
              <NumberFieldIncrement />
            </NumberFieldContent>
          </NumberField>

          <NumberField v-model="settings.filePort.value" :format-options="{ useGrouping: false }" :min="1" :max="65535">
            <Label>文件端口</Label>
            <NumberFieldContent>
              <NumberFieldDecrement />
              <NumberFieldInput />
              <NumberFieldIncrement />
            </NumberFieldContent>
          </NumberField>
        </div>

        <div class="space-y-2 rounded-md border p-3">
          <Button type="button" size="sm" variant="outline" class="gap-2" @click="chooseDownloadDir">
            <FolderOpen class="size-4" />
            选择下载目录
          </Button>
          <div class="truncate text-xs text-muted-foreground">
            {{ settings.downloadDir.value || '未设置，接收端启动前需要选择' }}
          </div>
        </div>

        <div class="rounded-md border p-3">
          <div class="flex items-start justify-between gap-3">
            <div class="min-w-0">
              <div class="flex items-center gap-2 text-sm font-medium">
                <Clipboard class="size-4" />
                文本剪贴板同步
              </div>
              <p class="mt-1 text-xs leading-5 text-muted-foreground">
                开启后，仅在鼠标移动到另一端或释放回本机时同步一次文本剪贴板。
              </p>
            </div>
            <button
              type="button"
              class="relative h-6 w-11 shrink-0 rounded-full border transition-colors"
              :class="settings.clipboardSharingEnabled.value ? 'border-primary bg-primary' : 'border-border bg-muted'"
              :aria-pressed="settings.clipboardSharingEnabled.value"
              @click="settings.clipboardSharingEnabled.value = !settings.clipboardSharingEnabled.value"
            >
              <span
                class="absolute top-0.5 size-5 rounded-full bg-background shadow-sm transition-transform"
                :class="settings.clipboardSharingEnabled.value ? 'left-5' : 'left-0.5'"
              />
            </button>
          </div>
        </div>

        <div class="rounded-md border border-dashed p-3">
          <div class="mb-2 flex items-center gap-2 text-sm font-medium">
            <Radar class="size-4" />
            局域网设备发现
          </div>
          <p class="text-xs leading-5 text-muted-foreground">
            已预留发现区块。下一步会通过局域网广播展示在线设备，并支持一键写入默认目标 IP。
          </p>
        </div>
      </section>

      <section class="min-h-0 rounded-lg border bg-background p-4">
        <div class="mb-3 flex items-center justify-between">
          <div>
            <h2 class="text-sm font-semibold">受信设备</h2>
            <p class="text-xs text-muted-foreground">首次确认并记住的设备会显示在这里。</p>
          </div>
          <span class="rounded-md border px-2 py-1 text-xs">{{ settings.trustedDevices.value.length }} 台</span>
        </div>

        <div v-if="settings.trustedDevices.value.length === 0" class="rounded-md border border-dashed p-6 text-center text-sm text-muted-foreground">
          暂无受信设备。未知设备连接时会弹出确认。
        </div>
        <div v-else class="space-y-2">
          <div
            v-for="device in settings.trustedDevices.value"
            :key="device.deviceId"
            class="flex items-center justify-between gap-3 rounded-md border p-3"
          >
            <div class="min-w-0">
              <div class="truncate text-sm font-medium">{{ device.deviceName }}</div>
              <div class="truncate text-xs text-muted-foreground">
                {{ device.lastIp || '未知 IP' }} · {{ device.deviceId }}
              </div>
            </div>
            <Button size="icon" variant="ghost" @click="settings.forgetDevice(device.deviceId)">
              <Trash2 class="size-4" />
            </Button>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>
