<script setup lang="ts">
import { type Ref, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'

const role = ref('server')
const ip = ref('')
const port = ref(4000)

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
    <div class="space-y-2">
      <SelectLabel>角色：</SelectLabel>
      <Select>
        <SelectTrigger class="w-[280px]">
          <SelectValue placeholder="请选择一个角色" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="server">控制端（发送）</SelectItem>
          <SelectItem value="client">客户端（接收）</SelectItem>
        </SelectContent>
      </Select>
    </div>

    <div v-if="role === 'server'" class="space-y-2">
      <Label>目标 IP：</Label>
      <Input v-model="ip" type="text" class="border px-2 py-1 rounded" placeholder="192.168.x.x" />
    </div>
    <div class="space-y-2">
      <Label Label>端口：</Label>
      <Input v-model.number="port" type="number" class="border px-2 py-1 rounded" placeholder="4000" />
    </div>

    <div class="space-x-8">
      <Button @click="start">
        启动共享
      </Button>
      <Button @click="stop" variant="destructive">
        停止共享
      </Button>
    </div>
  </div>
</template>

<style scoped></style>