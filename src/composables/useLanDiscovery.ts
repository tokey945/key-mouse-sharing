import { computed, ref } from 'vue'

export interface LanDevice {
  deviceId: string
  deviceName: string
  ip: string
  mousePort: number
  lastSeen: number
}

export interface LanConnectRequest {
  requestId: string
  deviceId: string
  deviceName: string
  peerIp: string
}

export interface LanConnectResponse {
  requestId: string
  accepted: boolean
  deviceId: string
  deviceName: string
  ip: string
  mousePort: number
  message?: string | null
}

const devices = ref<LanDevice[]>([])
const connectingDeviceId = ref<string | null>(null)
const pendingRequestId = ref<string | null>(null)
const connectionMessage = ref('')

export function upsertLanDevice(device: LanDevice) {
  const index = devices.value.findIndex((item) => item.deviceId === device.deviceId)
  if (index >= 0) devices.value.splice(index, 1, device)
  else devices.value.push(device)
  devices.value.sort((left, right) => left.deviceName.localeCompare(right.deviceName))
}

export function pruneLanDevices(maxAgeMs = 7_000) {
  const cutoff = Date.now() - maxAgeMs
  devices.value = devices.value.filter((device) => device.lastSeen >= cutoff)
}

export function useLanDiscovery() {
  return {
    devices,
    connectingDeviceId,
    pendingRequestId,
    connectionMessage,
    onlineCount: computed(() => devices.value.length),
  }
}

