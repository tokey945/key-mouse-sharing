<script setup lang="ts">
import { computed } from 'vue'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'

type LogLevel = 'info' | 'success' | 'warn' | 'error'
type LogFilter = 'all' | LogLevel

interface LogItem {
  at: number
  level: LogLevel
  message: string
}

const props = defineProps<{
  logs: LogItem[]
  filter: LogFilter
}>()

const emit = defineEmits<{
  'update:filter': [value: LogFilter]
  clear: []
}>()

const filters: Array<{ value: LogFilter; label: string }> = [
  { value: 'all', label: '全部' },
  { value: 'info', label: 'INFO' },
  { value: 'success', label: 'SUCCESS' },
  { value: 'warn', label: 'WARN' },
  { value: 'error', label: 'ERROR' },
]

const logStats = computed(() => {
  const stats = { all: props.logs.length, info: 0, success: 0, warn: 0, error: 0 }
  for (const item of props.logs) {
    stats[item.level] += 1
  }
  return stats
})

const filteredLogs = computed(() => {
  if (props.filter === 'all') return props.logs
  return props.logs.filter((log) => log.level === props.filter)
})

const getLogClass = (level: LogLevel) => {
  switch (level) {
    case 'error':
      return 'text-red-500'
    case 'warn':
      return 'text-amber-500'
    case 'success':
      return 'text-emerald-500'
    default:
      return 'text-foreground'
  }
}
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col rounded-lg border bg-background">
    <div class="flex flex-wrap items-center gap-2 border-b px-4 py-3">
      <div class="mr-2">
        <h2 class="text-sm font-semibold">运行日志</h2>
        <p class="text-xs text-muted-foreground">来自 Tauri 后端的实时状态</p>
      </div>
      <Button
        v-for="item in filters"
        :key="item.value"
        size="sm"
        :variant="filter === item.value ? 'default' : 'outline'"
        class="h-7 px-2 text-xs"
        @click="emit('update:filter', item.value)"
      >
        {{ item.label }} {{ logStats[item.value] }}
      </Button>
      <Button size="sm" variant="ghost" class="ml-auto h-7 px-2 text-xs" @click="emit('clear')">
        清空
      </Button>
    </div>

    <ScrollArea class="min-h-0 flex-1 p-4">
      <div class="space-y-1 font-mono text-xs leading-5">
        <div v-for="(log, index) in filteredLogs" :key="`${log.at}-${index}`" :class="getLogClass(log.level)">
          <span class="text-muted-foreground">{{ new Date(log.at).toLocaleTimeString() }}</span>
          <span class="mx-1">[{{ log.level.toUpperCase() }}]</span>
          <span>{{ log.message }}</span>
        </div>
        <div v-if="filteredLogs.length === 0" class="text-muted-foreground">
          暂无日志
        </div>
      </div>
    </ScrollArea>
  </section>
</template>
