<script setup lang="ts">
import { MonitorOff } from 'lucide-vue-next'
</script>
<template>
  <div class="flex h-full min-h-0 flex-col gap-4 p-4 pr-6">
    <header class="flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-background px-4 py-3">
      <div>
        <h1 class="text-base font-semibold">屏幕共享</h1>
        <p class="text-xs text-muted-foreground">本轮先稳定键鼠与文件共享，屏幕共享暂不开放。</p>
      </div>
      <div class="flex items-center gap-2 rounded-md border px-3 py-1.5 text-sm">
        <span class="size-2 rounded-full bg-muted-foreground" />
        暂未开放
      </div>
    </header>

    <section class="flex min-h-0 flex-1 items-center justify-center rounded-lg border bg-background">
      <div class="max-w-sm text-center">
        <div class="mx-auto mb-4 flex size-12 items-center justify-center rounded-lg border bg-muted">
          <MonitorOff class="size-6 text-muted-foreground" />
        </div>
        <h2 class="text-sm font-semibold">屏幕共享未纳入本轮实现</h2>
        <p class="mt-2 text-sm text-muted-foreground">
          当前版本不会展示误导性的启动控件；后续可以在这里接入低帧率预览或远控链路。
        </p>
      </div>
    </section>
  </div>
</template>
