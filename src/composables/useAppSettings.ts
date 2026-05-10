import { useStorage } from '@vueuse/core'

export interface DeviceIdentity {
  deviceId: string
  deviceName: string
}

export interface TrustedDevice extends DeviceIdentity {
  lastIp?: string
  trustedAt?: number
}

const createDeviceId = () => {
  const existing = localStorage.getItem('kms-device-id')
  if (existing) return existing.replace(/^"|"$/g, '')

  const random = globalThis.crypto?.randomUUID?.() ?? `device-${Date.now()}-${Math.random().toString(16).slice(2)}`
  localStorage.setItem('kms-device-id', JSON.stringify(random))
  return random
}

const defaultDeviceName = () => {
  const platform = navigator.platform || 'Desktop'
  return `${platform} 设备`
}

const deviceId = useStorage('kms-device-id', createDeviceId())
const deviceName = useStorage('kms-device-name', defaultDeviceName())
const defaultRole = useStorage<'server' | 'client'>('kms-default-role', 'server')
const targetIp = useStorage('kms-target-ip', '192.168.1.5')
const mousePort = useStorage('kms-mouse-port', 4000)
const filePort = useStorage('kms-file-port', 5001)
const downloadDir = useStorage('kms-download-dir', '')
const trustedDevices = useStorage<TrustedDevice[]>('kms-trusted-devices', [])
const clipboardSharingEnabled = useStorage('kms-clipboard-sharing-enabled', true)

export function useAppSettings() {
  const deviceIdentity = (): DeviceIdentity => ({
    deviceId: deviceId.value,
    deviceName: deviceName.value.trim() || defaultDeviceName(),
  })

  const trustDevice = (device: DeviceIdentity & { lastIp?: string }) => {
    const now = Date.now()
    const next: TrustedDevice = {
      deviceId: device.deviceId,
      deviceName: device.deviceName,
      lastIp: device.lastIp,
      trustedAt: now,
    }
    const index = trustedDevices.value.findIndex((item) => item.deviceId === device.deviceId)
    if (index >= 0) {
      trustedDevices.value.splice(index, 1, next)
    } else {
      trustedDevices.value.push(next)
    }
  }

  const forgetDevice = (deviceIdToForget: string) => {
    trustedDevices.value = trustedDevices.value.filter((item) => item.deviceId !== deviceIdToForget)
  }

  return {
    deviceId,
    deviceName,
    defaultRole,
    targetIp,
    mousePort,
    filePort,
    downloadDir,
    trustedDevices,
    clipboardSharingEnabled,
    deviceIdentity,
    trustDevice,
    forgetDevice,
  }
}
