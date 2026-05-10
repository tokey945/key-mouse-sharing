import { ref } from 'vue'

export interface ConnectionState {
  connected: boolean
  peerIp?: string | null
  peerDeviceId?: string | null
  peerDeviceName?: string | null
}

export const connectionState = ref<ConnectionState>({
  connected: false,
  peerIp: null,
  peerDeviceId: null,
  peerDeviceName: null,
})

export function setConnectionState(state: ConnectionState) {
  connectionState.value = {
    connected: state.connected,
    peerIp: state.peerIp ?? null,
    peerDeviceId: state.peerDeviceId ?? null,
    peerDeviceName: state.peerDeviceName ?? null,
  }
}

export function useConnectionState() {
  return { connectionState, setConnectionState }
}
