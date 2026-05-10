<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  Binary,
  Braces,
  CalendarClock,
  Clock3,
  Fingerprint,
  KeyRound,
  Regex,
  RotateCcw,
  ShieldCheck,
  TerminalSquare,
} from 'lucide-vue-next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

type ToolId = 'timestamp' | 'json' | 'codec' | 'hash' | 'aes' | 'jwt' | 'random' | 'regex' | 'cron' | 'linux'

interface ToolItem {
  id: ToolId
  title: string
  description: string
  icon: any
}

const tools: ToolItem[] = [
  { id: 'timestamp', title: '时间戳', description: '秒/毫秒、ISO、本地时间互转', icon: Clock3 },
  { id: 'json', title: 'JSON', description: '格式化、压缩、校验', icon: Braces },
  { id: 'codec', title: '编码转换', description: 'Base64 与 URL 编解码', icon: Binary },
  { id: 'hash', title: 'Hash', description: 'SHA 摘要计算', icon: Fingerprint },
  { id: 'aes', title: 'AES-GCM', description: '本地临时加解密', icon: KeyRound },
  { id: 'jwt', title: 'JWT', description: '解析 Header 与 Payload', icon: ShieldCheck },
  { id: 'random', title: 'UUID/随机值', description: 'UUID、Hex、Base64', icon: RotateCcw },
  { id: 'regex', title: '正则测试', description: '匹配与捕获组检查', icon: Regex },
  { id: 'cron', title: 'Cron', description: '5 段表达式解释与预览', icon: CalendarClock },
  { id: 'linux', title: 'Linux 命令', description: '常用命令速查', icon: TerminalSquare },
]

const activeTool = ref<ToolId>('timestamp')
const status = ref('')
const error = ref('')

const currentTool = computed(() => tools.find((tool) => tool.id === activeTool.value) ?? tools[0])

const textEncoder = new TextEncoder()
const textDecoder = new TextDecoder()

const setStatus = (message: string) => {
  status.value = message
  error.value = ''
}

const setError = (message: string) => {
  error.value = message
  status.value = ''
}

const copyText = async (value: string) => {
  if (!value) return
  await navigator.clipboard.writeText(value)
  setStatus('已复制到剪贴板')
}

const bytesToBase64 = (bytes: Uint8Array) => {
  let binary = ''
  bytes.forEach((byte) => {
    binary += String.fromCharCode(byte)
  })
  return btoa(binary)
}

const base64ToBytes = (base64: string) => {
  const binary = atob(base64)
  return Uint8Array.from(binary, (char) => char.charCodeAt(0))
}

const base64UrlDecode = (value: string) => {
  const base64 = value.replace(/-/g, '+').replace(/_/g, '/').padEnd(Math.ceil(value.length / 4) * 4, '=')
  return textDecoder.decode(base64ToBytes(base64))
}

const toHex = (bytes: Uint8Array) => [...bytes].map((byte) => byte.toString(16).padStart(2, '0')).join('')

// Timestamp
const timestampInput = ref(String(Date.now()))
const timestampNow = ref(new Date())
const timestampResult = computed(() => {
  const raw = timestampInput.value.trim()
  const numeric = Number(raw)
  if (!raw || Number.isNaN(numeric)) return '请输入秒或毫秒时间戳'
  const ms = raw.length <= 10 ? numeric * 1000 : numeric
  const date = new Date(ms)
  if (Number.isNaN(date.getTime())) return '时间戳无效'
  return [
    `毫秒: ${date.getTime()}`,
    `秒: ${Math.floor(date.getTime() / 1000)}`,
    `ISO: ${date.toISOString()}`,
    `本地: ${date.toLocaleString()}`,
  ].join('\n')
})

const useCurrentTime = () => {
  timestampNow.value = new Date()
  timestampInput.value = String(timestampNow.value.getTime())
}

// JSON
const jsonInput = ref('{"name":"key-mouse-sharing","features":["keyboard","mouse","file"]}')
const jsonOutput = ref('')
const jsonMode = ref<'format' | 'minify'>('format')

const transformJson = () => {
  try {
    const parsed = JSON.parse(jsonInput.value)
    jsonOutput.value = jsonMode.value === 'format' ? JSON.stringify(parsed, null, 2) : JSON.stringify(parsed)
    setStatus('JSON 处理完成')
  } catch (err: any) {
    setError(`JSON 无效: ${err?.message ?? err}`)
  }
}

// Codec
const codecInput = ref('Hello, 程序员')
const codecOutput = ref('')
const codecMode = ref<'base64-encode' | 'base64-decode' | 'url-encode' | 'url-decode'>('base64-encode')

const transformCodec = () => {
  try {
    if (codecMode.value === 'base64-encode') codecOutput.value = bytesToBase64(textEncoder.encode(codecInput.value))
    if (codecMode.value === 'base64-decode') codecOutput.value = textDecoder.decode(base64ToBytes(codecInput.value.trim()))
    if (codecMode.value === 'url-encode') codecOutput.value = encodeURIComponent(codecInput.value)
    if (codecMode.value === 'url-decode') codecOutput.value = decodeURIComponent(codecInput.value)
    setStatus('编码转换完成')
  } catch (err: any) {
    setError(`转换失败: ${err?.message ?? err}`)
  }
}

// Hash
const hashInput = ref('hello')
const hashAlgo = ref<'SHA-1' | 'SHA-256' | 'SHA-384' | 'SHA-512'>('SHA-256')
const hashOutput = ref('')

const calculateHash = async () => {
  if (!crypto.subtle) {
    setError('当前环境不支持 Web Crypto')
    return
  }
  const digest = await crypto.subtle.digest(hashAlgo.value, textEncoder.encode(hashInput.value))
  hashOutput.value = toHex(new Uint8Array(digest))
  setStatus('Hash 计算完成')
}

// AES
const aesInput = ref('local secret')
const aesPassword = ref('dev-password')
const aesOutput = ref('')

const deriveAesKey = async (password: string, salt: Uint8Array) => {
  const material = await crypto.subtle.importKey('raw', textEncoder.encode(password), 'PBKDF2', false, ['deriveKey'])
  return crypto.subtle.deriveKey(
    { name: 'PBKDF2', salt, iterations: 100_000, hash: 'SHA-256' },
    material,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt'],
  )
}

const encryptAes = async () => {
  try {
    const salt = crypto.getRandomValues(new Uint8Array(16))
    const iv = crypto.getRandomValues(new Uint8Array(12))
    const key = await deriveAesKey(aesPassword.value, salt)
    const encrypted = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, textEncoder.encode(aesInput.value))
    aesOutput.value = bytesToBase64(textEncoder.encode(JSON.stringify({
      v: 1,
      alg: 'AES-GCM',
      salt: bytesToBase64(salt),
      iv: bytesToBase64(iv),
      data: bytesToBase64(new Uint8Array(encrypted)),
    })))
    setStatus('AES-GCM 加密完成')
  } catch (err: any) {
    setError(`加密失败: ${err?.message ?? err}`)
  }
}

const decryptAes = async () => {
  try {
    const packed = JSON.parse(textDecoder.decode(base64ToBytes(aesInput.value.trim())))
    const salt = base64ToBytes(packed.salt)
    const iv = base64ToBytes(packed.iv)
    const data = base64ToBytes(packed.data)
    const key = await deriveAesKey(aesPassword.value, salt)
    const decrypted = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, key, data)
    aesOutput.value = textDecoder.decode(decrypted)
    setStatus('AES-GCM 解密完成')
  } catch {
    setError('解密失败：密文格式、口令或数据可能不正确')
  }
}

// JWT
const jwtInput = ref('eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjMiLCJuYW1lIjoiQ29kZXgiLCJleHAiOjQxMDI0NDQ4MDB9.signature')
const jwtOutput = ref('')

const parseJwt = () => {
  try {
    const parts = jwtInput.value.trim().split('.')
    if (parts.length !== 3) throw new Error('JWT 必须是三段结构')
    const header = JSON.parse(base64UrlDecode(parts[0]))
    const payload = JSON.parse(base64UrlDecode(parts[1]))
    const exp = typeof payload.exp === 'number' ? new Date(payload.exp * 1000).toLocaleString() : '无'
    jwtOutput.value = JSON.stringify({ header, payload, expiresAt: exp, signaturePreview: parts[2].slice(0, 16) }, null, 2)
    setStatus('JWT 解析完成；未验证签名')
  } catch (err: any) {
    setError(`JWT 无效: ${err?.message ?? err}`)
  }
}

// Random
const randomLength = ref(32)
const randomOutput = ref('')

const generateUuid = () => {
  randomOutput.value = crypto.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`
  setStatus('UUID 已生成')
}

const generateRandom = (mode: 'hex' | 'base64') => {
  const bytes = crypto.getRandomValues(new Uint8Array(Math.max(1, Math.min(256, Number(randomLength.value)))))
  randomOutput.value = mode === 'hex' ? toHex(bytes) : bytesToBase64(bytes)
  setStatus(`随机 ${mode} 已生成`)
}

// Regex
const regexPattern = ref('\\b\\w+@\\w+\\.\\w+\\b')
const regexFlags = ref('gi')
const regexText = ref('Contact dev@example.com or ops@example.com')
const regexOutput = ref('')

const testRegex = () => {
  try {
    const safeFlags = regexFlags.value.includes('g') ? regexFlags.value : `${regexFlags.value}g`
    const re = new RegExp(regexPattern.value, safeFlags)
    const matches = [...regexText.value.matchAll(re)]
    regexOutput.value = matches.length
      ? matches.map((match, index) => `#${index + 1} ${match[0]} @ ${match.index}\n捕获组: ${JSON.stringify(match.slice(1))}`).join('\n\n')
      : '没有匹配'
    setStatus(`匹配完成，共 ${matches.length} 条`)
  } catch (err: any) {
    setError(`正则无效: ${err?.message ?? err}`)
  }
}

// Cron
const cronInput = ref('*/5 * * * *')
const cronOutput = ref('')

const parseCronPart = (part: string, min: number, max: number) => {
  const values = new Set<number>()
  for (const token of part.split(',')) {
    if (token === '*') {
      for (let i = min; i <= max; i++) values.add(i)
      continue
    }
    if (token.startsWith('*/')) {
      const step = Number(token.slice(2))
      for (let i = min; i <= max; i += step) values.add(i)
      continue
    }
    const [startRaw, endRaw] = token.split('-')
    if (endRaw !== undefined) {
      for (let i = Number(startRaw); i <= Number(endRaw); i++) values.add(i)
      continue
    }
    values.add(Number(token))
  }
  return [...values].filter((value) => Number.isInteger(value) && value >= min && value <= max)
}

const explainCronPart = (part: string, label: string) => {
  if (part === '*') return `${label}: 每个值`
  if (part.startsWith('*/')) return `${label}: 每 ${part.slice(2)} 个单位`
  return `${label}: ${part}`
}

const previewCron = () => {
  try {
    const parts = cronInput.value.trim().split(/\s+/)
    if (parts.length !== 5) throw new Error('仅支持 5 段 cron: 分 时 日 月 周')
    const [minutePart, hourPart, dayPart, monthPart, weekPart] = parts
    const minutes = parseCronPart(minutePart, 0, 59)
    const hours = parseCronPart(hourPart, 0, 23)
    const days = parseCronPart(dayPart, 1, 31)
    const months = parseCronPart(monthPart, 1, 12)
    const weeks = parseCronPart(weekPart, 0, 7).map((week) => week === 7 ? 0 : week)
    if ([minutes, hours, days, months, weeks].some((list) => list.length === 0)) throw new Error('表达式包含无效字段')

    const hits: Date[] = []
    const cursor = new Date()
    cursor.setSeconds(0, 0)
    for (let i = 0; i < 60 * 24 * 370 && hits.length < 5; i++) {
      cursor.setMinutes(cursor.getMinutes() + 1)
      if (
        minutes.includes(cursor.getMinutes()) &&
        hours.includes(cursor.getHours()) &&
        days.includes(cursor.getDate()) &&
        months.includes(cursor.getMonth() + 1) &&
        weeks.includes(cursor.getDay())
      ) {
        hits.push(new Date(cursor))
      }
    }

    cronOutput.value = [
      explainCronPart(minutePart, '分钟'),
      explainCronPart(hourPart, '小时'),
      explainCronPart(dayPart, '日期'),
      explainCronPart(monthPart, '月份'),
      explainCronPart(weekPart, '星期'),
      '',
      '接下来 5 次:',
      ...hits.map((hit) => hit.toLocaleString()),
    ].join('\n')
    setStatus('Cron 预览完成')
  } catch (err: any) {
    setError(`Cron 无效: ${err?.message ?? err}`)
  }
}

// Linux commands
const linuxQuery = ref('')
const linuxCommands = [
  { group: '文件', cmd: 'ls -la', desc: '显示详细文件列表' },
  { group: '文件', cmd: 'find . -name "*.log"', desc: '按文件名查找' },
  { group: '文件', cmd: 'du -sh *', desc: '查看当前目录各项大小' },
  { group: '文本', cmd: 'grep -R "keyword" .', desc: '递归搜索文本' },
  { group: '文本', cmd: 'sed -n "1,120p" file', desc: '查看文件指定行范围' },
  { group: '压缩', cmd: 'tar -czf app.tar.gz app/', desc: '打包 gzip 压缩' },
  { group: '压缩', cmd: 'tar -xzf app.tar.gz', desc: '解压 tar.gz' },
  { group: '进程', cmd: 'ps aux | grep node', desc: '查找进程' },
  { group: '进程', cmd: 'kill -9 <pid>', desc: '强制结束进程' },
  { group: '网络', cmd: 'lsof -i :3000', desc: '查看端口占用' },
  { group: '网络', cmd: 'curl -I https://example.com', desc: '查看响应头' },
  { group: '网络', cmd: 'nc -vz 127.0.0.1 4000', desc: '探测 TCP 端口' },
  { group: '权限', cmd: 'chmod +x script.sh', desc: '添加可执行权限' },
  { group: '权限', cmd: 'chown -R user:group dir', desc: '修改所有者' },
  { group: '磁盘', cmd: 'df -h', desc: '查看磁盘空间' },
  { group: 'Git', cmd: 'git status --short', desc: '查看简短状态' },
  { group: 'Git', cmd: 'git log --oneline -n 10', desc: '查看最近提交' },
]

const filteredLinuxCommands = computed(() => {
  const q = linuxQuery.value.trim().toLowerCase()
  if (!q) return linuxCommands
  return linuxCommands.filter((item) => `${item.group} ${item.cmd} ${item.desc}`.toLowerCase().includes(q))
})
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-4 p-4 pr-6">
    <header class="flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-background px-4 py-3">
      <div>
        <h1 class="text-base font-semibold">程序员工具箱</h1>
        <p class="text-xs text-muted-foreground">离线常用工具：时间、JSON、编码、Hash、AES、JWT、正则、Cron 与 Linux 命令。</p>
      </div>
      <div class="rounded-md border px-3 py-1.5 text-xs text-muted-foreground">
        {{ currentTool.title }}
      </div>
    </header>

    <div class="grid min-h-0 flex-1 gap-4 xl:grid-cols-[260px_minmax(0,1fr)]">
      <aside class="min-h-0 rounded-lg border bg-background p-2">
        <button
          v-for="tool in tools"
          :key="tool.id"
          type="button"
          class="mb-1 flex w-full items-center gap-3 rounded-md px-3 py-2 text-left transition-colors hover:bg-accent"
          :class="activeTool === tool.id ? 'bg-accent text-accent-foreground' : ''"
          @click="activeTool = tool.id; status = ''; error = ''"
        >
          <component :is="tool.icon" class="size-4 shrink-0" />
          <span class="min-w-0">
            <span class="block text-sm font-medium">{{ tool.title }}</span>
            <span class="block truncate text-xs text-muted-foreground">{{ tool.description }}</span>
          </span>
        </button>
      </aside>

      <main class="min-h-0 rounded-lg border bg-background p-4">
        <div v-if="status" class="mb-3 rounded-md border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-900 dark:border-emerald-900/50 dark:bg-emerald-950/30 dark:text-emerald-200">{{ status }}</div>
        <div v-if="error" class="mb-3 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-900 dark:border-red-900/50 dark:bg-red-950/30 dark:text-red-200">{{ error }}</div>

        <section v-if="activeTool === 'timestamp'" class="space-y-4">
          <div class="grid gap-2">
            <Label>时间戳</Label>
            <Input v-model="timestampInput" />
          </div>
          <div class="flex gap-2">
            <Button @click="useCurrentTime">使用当前时间</Button>
            <Button variant="outline" @click="copyText(timestampResult)">复制结果</Button>
          </div>
          <pre class="min-h-40 overflow-auto rounded-md border bg-muted/40 p-3 text-sm">{{ timestampResult }}</pre>
        </section>

        <section v-if="activeTool === 'json'" class="grid min-h-0 gap-4 lg:grid-cols-2">
          <div class="space-y-3">
            <div class="flex gap-2">
              <Button variant="outline" @click="jsonMode = 'format'; transformJson()">格式化</Button>
              <Button variant="outline" @click="jsonMode = 'minify'; transformJson()">压缩</Button>
            </div>
            <textarea v-model="jsonInput" class="h-[460px] w-full resize-none rounded-md border bg-background p-3 font-mono text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring" />
          </div>
          <div class="space-y-3">
            <Button variant="outline" @click="copyText(jsonOutput)">复制结果</Button>
            <textarea v-model="jsonOutput" readonly class="h-[460px] w-full resize-none rounded-md border bg-muted/40 p-3 font-mono text-sm outline-none" />
          </div>
        </section>

        <section v-if="activeTool === 'codec'" class="grid gap-4 lg:grid-cols-2">
          <div class="space-y-3">
            <select v-model="codecMode" class="h-9 rounded-md border bg-background px-3 text-sm">
              <option value="base64-encode">Base64 编码</option>
              <option value="base64-decode">Base64 解码</option>
              <option value="url-encode">URL Encode</option>
              <option value="url-decode">URL Decode</option>
            </select>
            <textarea v-model="codecInput" class="h-72 w-full resize-none rounded-md border bg-background p-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring" />
            <Button @click="transformCodec">转换</Button>
          </div>
          <div class="space-y-3">
            <Button variant="outline" @click="copyText(codecOutput)">复制结果</Button>
            <textarea v-model="codecOutput" readonly class="h-72 w-full resize-none rounded-md border bg-muted/40 p-3 text-sm outline-none" />
          </div>
        </section>

        <section v-if="activeTool === 'hash'" class="space-y-4">
          <select v-model="hashAlgo" class="h-9 rounded-md border bg-background px-3 text-sm">
            <option>SHA-1</option>
            <option>SHA-256</option>
            <option>SHA-384</option>
            <option>SHA-512</option>
          </select>
          <textarea v-model="hashInput" class="h-48 w-full resize-none rounded-md border bg-background p-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring" />
          <div class="flex gap-2">
            <Button @click="calculateHash">计算</Button>
            <Button variant="outline" @click="copyText(hashOutput)">复制</Button>
          </div>
          <pre class="overflow-auto rounded-md border bg-muted/40 p-3 font-mono text-sm">{{ hashOutput }}</pre>
        </section>

        <section v-if="activeTool === 'aes'" class="grid gap-4 lg:grid-cols-2">
          <div class="space-y-3">
            <div class="rounded-md border border-amber-200 bg-amber-50 p-3 text-xs text-amber-900 dark:border-amber-900/50 dark:bg-amber-950/30 dark:text-amber-200">仅用于本地临时调试，不要当作正式密钥管理工具。</div>
            <Input v-model="aesPassword" placeholder="口令" />
            <textarea v-model="aesInput" class="h-72 w-full resize-none rounded-md border bg-background p-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring" />
            <div class="flex gap-2">
              <Button @click="encryptAes">加密</Button>
              <Button variant="outline" @click="decryptAes">解密</Button>
            </div>
          </div>
          <div class="space-y-3">
            <Button variant="outline" @click="copyText(aesOutput)">复制结果</Button>
            <textarea v-model="aesOutput" readonly class="h-96 w-full resize-none rounded-md border bg-muted/40 p-3 text-sm outline-none" />
          </div>
        </section>

        <section v-if="activeTool === 'jwt'" class="grid gap-4 lg:grid-cols-2">
          <textarea v-model="jwtInput" class="h-96 w-full resize-none rounded-md border bg-background p-3 font-mono text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring" />
          <div class="space-y-3">
            <div class="flex gap-2">
              <Button @click="parseJwt">解析</Button>
              <Button variant="outline" @click="copyText(jwtOutput)">复制</Button>
            </div>
            <textarea v-model="jwtOutput" readonly class="h-[342px] w-full resize-none rounded-md border bg-muted/40 p-3 font-mono text-sm outline-none" />
          </div>
        </section>

        <section v-if="activeTool === 'random'" class="space-y-4">
          <div class="grid max-w-xs gap-2">
            <Label>字节长度</Label>
            <Input v-model.number="randomLength" type="number" min="1" max="256" />
          </div>
          <div class="flex flex-wrap gap-2">
            <Button @click="generateUuid">UUID v4</Button>
            <Button variant="outline" @click="generateRandom('hex')">随机 Hex</Button>
            <Button variant="outline" @click="generateRandom('base64')">随机 Base64</Button>
            <Button variant="outline" @click="copyText(randomOutput)">复制</Button>
          </div>
          <pre class="overflow-auto rounded-md border bg-muted/40 p-3 font-mono text-sm">{{ randomOutput }}</pre>
        </section>

        <section v-if="activeTool === 'regex'" class="space-y-4">
          <div class="grid gap-3 lg:grid-cols-[1fr_120px]">
            <Input v-model="regexPattern" placeholder="Pattern" />
            <Input v-model="regexFlags" placeholder="flags" />
          </div>
          <textarea v-model="regexText" class="h-48 w-full resize-none rounded-md border bg-background p-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring" />
          <div class="flex gap-2">
            <Button @click="testRegex">测试</Button>
            <Button variant="outline" @click="copyText(regexOutput)">复制</Button>
          </div>
          <pre class="min-h-40 overflow-auto rounded-md border bg-muted/40 p-3 font-mono text-sm">{{ regexOutput }}</pre>
        </section>

        <section v-if="activeTool === 'cron'" class="space-y-4">
          <Input v-model="cronInput" placeholder="*/5 * * * *" />
          <div class="flex gap-2">
            <Button @click="previewCron">解释并预览</Button>
            <Button variant="outline" @click="copyText(cronOutput)">复制</Button>
          </div>
          <pre class="min-h-64 overflow-auto rounded-md border bg-muted/40 p-3 font-mono text-sm">{{ cronOutput }}</pre>
        </section>

        <section v-if="activeTool === 'linux'" class="space-y-4">
          <Input v-model="linuxQuery" placeholder="搜索：网络、权限、git、端口..." />
          <div class="grid gap-2 md:grid-cols-2 xl:grid-cols-3">
            <button
              v-for="item in filteredLinuxCommands"
              :key="item.cmd"
              type="button"
              class="rounded-md border bg-background p-3 text-left hover:bg-accent"
              @click="copyText(item.cmd)"
            >
              <div class="mb-1 text-xs text-muted-foreground">{{ item.group }}</div>
              <code class="block break-all font-mono text-sm">{{ item.cmd }}</code>
              <div class="mt-2 text-xs text-muted-foreground">{{ item.desc }}</div>
            </button>
          </div>
        </section>
      </main>
    </div>
  </div>
</template>
