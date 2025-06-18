<template>
  <div class="p-4 space-y-4">
    <div class="space-y-2">
      <label class="block font-medium text-red-600">角色：</label>
      <select v-model="role" class="border px-2 py-1 rounded">
        <option value="server">控制端（发送）</option>
        <option value="client">客户端（接收）</option>
      </select>
    </div>

    <div v-if="role === 'server'" class="space-y-2">
      <label>目标 IP：</label>
      <input v-model="ip" type="text" class="border px-2 py-1 rounded" placeholder="192.168.x.x" />
    </div>

    <div class="space-y-2">
      <label>端口：</label>
      <input v-model.number="port" type="number" class="border px-2 py-1 rounded" placeholder="4000" />
    </div>

    <div class="space-x-4">
      <button @click="start" class="bg-blue-500 text-white px-4 py-2 rounded hover:bg-blue-600">
        启动共享
      </button>
      <button @click="stop" class="bg-red-500 text-white px-4 py-2 rounded hover:bg-red-600">
        停止共享
      </button>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core';
import { message } from '@tauri-apps/plugin-dialog';

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
  } catch (error) {
    console.error('启动失败:', error)
    alert(`启动失败: ${error.message || '未知错误'}`)
  }
}

const stop = async () => {
  try {
    await invoke('stop_sharing')
    alert('已停止共享')
  } catch (error) {
    console.error('停止失败:', error)
    alert(`停止失败: ${error.message || '未知错误'}`)
  }
}
</script>

<style scoped>
input,
select {
  width: 200px;
}
</style>