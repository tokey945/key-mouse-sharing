<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import {
  NumberField,
  NumberFieldContent,
  NumberFieldDecrement,
  NumberFieldIncrement,
  NumberFieldInput,
} from '@/components/ui/number-field'

const role = ref('server')
const ip = ref('192.168.1.5')
const port = ref(4000)


// 模拟的日志数据
const logs = ref([
  { level: 'info', message: '服务启动成功，端口：3000' },
  { level: 'info', message: '正在连接数据库...' },
  { level: 'success', message: '数据库连接成功！' },
  { level: 'warn', message: '警告：内存使用率达到 85%' },
  { level: 'error', message: '错误：无法写入文件 /var/log/app.log' },
  { level: 'info', message: '接收到新的请求 from 192.168.1.10' },
  { level: 'info', message: '请求处理完成，耗时 120ms' },
  { level: 'info', message: '请求处理完成，耗时 120ms' },
  { level: 'info', message: '请求处理完成，耗时 120ms' },
  { level: 'info', message: '请求处理完成，耗时 120ms' },
  { level: 'info', message: '请求处理完成，耗时 120ms' },
  { level: 'info', message: '请求处理完成，耗时 120ms' },
  { level: 'info', message: '请求处理完成，耗时 120ms' },
  { level: 'info', message: '请求处理完成，耗时 120ms' },
  { level: 'info', message: '请求处理完成，耗时 120ms' },
  { level: 'info', message: '请求处理完成，耗时 120ms' },
  // ...更多日志
])

// 用于根据日志级别返回不同颜色的类
const getLogClass = (level: string) => {
  switch (level) {
    case 'error': return 'text-red-500'
    case 'warn': return 'text-yellow-500'
    case 'success': return 'text-green-500'
    default: return ''
  }
}

const start = async () => {
  try {
    if (role.value === 'server') {
      if (!ip.value) {
        await message('请输入目标 IP 地址');
        return
      }
      await invoke('start_mouse_server', { ip: ip.value, port: port.value })
      await message('控制端已启动');
    } else {
      await invoke('start_mouse_client', { port: port.value })
      await message('客户端已启动');
    }
  } catch (error: any) {
    console.error('启动失败:', error)
    alert(`启动失败: ${error.message || '未知错误'}`)
  }
}

const stop = async () => {
  try {
    await invoke('stop_sharing')
    alert('已停止共享')
  } catch (error: any) {
    console.error('停止失败:', error)
    alert(`停止失败: ${error.message || '未知错误'}`)
  }
}
</script>

<template>
  <div class="p-4 space-y-4">
    <Label>本机角色：</Label>
    <RadioGroup v-model="role" default-value="server" :orientation="'horizontal'" class="flex space-x-4">
      <div class="flex items-center space-x-2">
        <RadioGroupItem id="r1" value="server" />
        <Label for="r1">控制端（发送）</Label>
      </div>
      <div class="flex items-center space-x-2">
        <RadioGroupItem id="r2" value="client" />
        <Label for="r2">客户端（接收）</Label>
      </div>
    </RadioGroup>
    <div class="grid w-full max-w-sm items-center gap-1.5">
      <Label for="email">目标 IP：</Label>
      <Input v-model="ip" type="text" class="w-40  focus-visible:ring-0 " />
    </div>
    <NumberField id="age" v-model="port" :formatOptions="{ useGrouping: false }" :min="0" class="w-40">
      <Label for="age">端口：</Label>
      <NumberFieldContent>
        <NumberFieldDecrement />
        <NumberFieldInput />
        <NumberFieldIncrement />
      </NumberFieldContent>
    </NumberField>
    <div class="space-x-8">
      <Button @click="start"> 启动共享</Button>
      <Button @click="stop" variant="destructive"> 停止共享</Button>
    </div>
  </div>
  <!-- 日志输出 -->
  <div class="w-full max-w-2xl h-50 p-0 m-auto">
    <h6 class="text-lg font-semibold mb-2">logs</h6>
    <!-- 设置一个固定高度，并用 ScrollArea 包裹 -->
    <ScrollArea class="w-full h-full rounded-md border p-4 bg-accent/30">
      <!-- 使用 pre 标签和等宽字体 -->
      <div class="font-mono text-sm">
        <div v-for="(log, index) in logs" :key="index" :class="getLogClass(log.level)">
          {{ new Date().toLocaleTimeString() }} [{{ log.level.toUpperCase() }}]: {{ log.message }}
        </div>
      </div>
    </ScrollArea>
  </div>
</template>

<style scoped></style>