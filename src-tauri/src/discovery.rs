use crate::mouse_share::{allow_runtime_discovered_device, DeviceIdentity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

const DISCOVERY_PORT: u16 = 47_821;
const ANNOUNCE_INTERVAL: Duration = Duration::from_secs(2);

static DISCOVERY_RUNNING: AtomicBool = AtomicBool::new(false);

lazy_static::lazy_static! {
    static ref LOCAL_IDENTITY: Mutex<Option<DeviceIdentity>> = Mutex::new(None);
    static ref LOCAL_MOUSE_PORT: Mutex<u16> = Mutex::new(4000);
    static ref PENDING_REQUESTS: Mutex<HashMap<String, PendingConnectionRequest>> = Mutex::new(HashMap::new());
}

#[derive(Clone)]
struct PendingConnectionRequest {
    requester: DeviceIdentity,
    peer_ip: IpAddr,
    created_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum DiscoveryMessage {
    Announce {
        #[serde(flatten)]
        identity: DeviceIdentity,
        #[serde(rename = "mousePort")]
        mouse_port: u16,
    },
    ConnectRequest {
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "targetDeviceId")]
        target_device_id: String,
        requester: DeviceIdentity,
    },
    ConnectResponse {
        #[serde(rename = "requestId")]
        request_id: String,
        accepted: bool,
        responder: DeviceIdentity,
        #[serde(rename = "mousePort")]
        mouse_port: u16,
        message: Option<String>,
    },
}

#[derive(Clone, Serialize)]
struct DiscoveredDeviceEvent {
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "deviceName")]
    device_name: String,
    ip: String,
    #[serde(rename = "mousePort")]
    mouse_port: u16,
    #[serde(rename = "lastSeen")]
    last_seen: u64,
}

#[derive(Clone, Serialize)]
struct IncomingConnectionRequestEvent {
    #[serde(rename = "requestId")]
    request_id: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "deviceName")]
    device_name: String,
    #[serde(rename = "peerIp")]
    peer_ip: String,
}

#[derive(Clone, Serialize)]
struct ConnectionResponseEvent {
    #[serde(rename = "requestId")]
    request_id: String,
    accepted: bool,
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "deviceName")]
    device_name: String,
    ip: String,
    #[serde(rename = "mousePort")]
    mouse_port: u16,
    message: Option<String>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn local_identity() -> Result<DeviceIdentity, String> {
    LOCAL_IDENTITY
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "局域网发现服务尚未配置本机身份".to_string())
}

fn send_message(
    socket: &UdpSocket,
    target: SocketAddr,
    message: &DiscoveryMessage,
) -> Result<(), String> {
    let payload = serde_json::to_vec(message).map_err(|e| format!("发现消息序列化失败: {e}"))?;
    socket
        .send_to(&payload, target)
        .map_err(|e| format!("发现消息发送失败: {e}"))?;
    Ok(())
}

fn broadcast_announcement(socket: &UdpSocket) -> Result<(), String> {
    let identity = local_identity()?;
    let mouse_port = *LOCAL_MOUSE_PORT.lock().unwrap();
    let message = DiscoveryMessage::Announce {
        identity,
        mouse_port,
    };
    send_message(
        socket,
        SocketAddr::from(([255, 255, 255, 255], DISCOVERY_PORT)),
        &message,
    )
}

fn process_message(
    app: &AppHandle,
    socket: &UdpSocket,
    source: SocketAddr,
    message: DiscoveryMessage,
) {
    let Ok(local) = local_identity() else {
        return;
    };

    match message {
        DiscoveryMessage::Announce {
            identity,
            mouse_port,
        } => {
            if identity.device_id == local.device_id {
                return;
            }
            let _ = app.emit(
                "lan-device-upsert",
                DiscoveredDeviceEvent {
                    device_id: identity.device_id,
                    device_name: identity.device_name,
                    ip: source.ip().to_string(),
                    mouse_port,
                    last_seen: now_ms(),
                },
            );
        }
        DiscoveryMessage::ConnectRequest {
            request_id,
            target_device_id,
            requester,
        } => {
            if target_device_id != local.device_id || requester.device_id == local.device_id {
                return;
            }
            if requester.device_id.trim().len() < 8 || requester.device_name.trim().is_empty() {
                return;
            }
            let mut pending_requests = PENDING_REQUESTS.lock().unwrap();
            pending_requests
                .retain(|_, pending| now_ms().saturating_sub(pending.created_at) < 60_000);
            pending_requests.insert(
                request_id.clone(),
                PendingConnectionRequest {
                    requester: requester.clone(),
                    peer_ip: source.ip(),
                    created_at: now_ms(),
                },
            );
            drop(pending_requests);
            let _ = app.emit(
                "lan-connect-request",
                IncomingConnectionRequestEvent {
                    request_id,
                    device_id: requester.device_id,
                    device_name: requester.device_name,
                    peer_ip: source.ip().to_string(),
                },
            );
        }
        DiscoveryMessage::ConnectResponse {
            request_id,
            accepted,
            responder,
            mouse_port,
            message,
        } => {
            let _ = app.emit(
                "lan-connect-response",
                ConnectionResponseEvent {
                    request_id,
                    accepted,
                    device_id: responder.device_id,
                    device_name: responder.device_name,
                    ip: source.ip().to_string(),
                    mouse_port,
                    message,
                },
            );
        }
    }

    let _ = socket;
}

#[tauri::command]
pub fn start_device_discovery(
    app: AppHandle,
    device_identity: DeviceIdentity,
    mouse_port: u16,
) -> Result<(), String> {
    if device_identity.device_id.trim().len() < 8 || device_identity.device_name.trim().is_empty() {
        return Err("本机设备身份无效".to_string());
    }
    *LOCAL_IDENTITY.lock().unwrap() = Some(device_identity);
    *LOCAL_MOUSE_PORT.lock().unwrap() = mouse_port;

    if DISCOVERY_RUNNING.load(Ordering::SeqCst) {
        return Ok(());
    }

    let socket = UdpSocket::bind(("0.0.0.0", DISCOVERY_PORT))
        .map_err(|e| format!("局域网发现端口 {DISCOVERY_PORT} 绑定失败: {e}"))?;
    socket
        .set_broadcast(true)
        .map_err(|e| format!("开启 UDP 广播失败: {e}"))?;
    socket
        .set_read_timeout(Some(Duration::from_millis(500)))
        .map_err(|e| format!("设置发现服务超时失败: {e}"))?;
    DISCOVERY_RUNNING.store(true, Ordering::SeqCst);

    thread::spawn(move || {
        let mut last_announce = SystemTime::UNIX_EPOCH;
        let mut buffer = [0_u8; 8192];
        while DISCOVERY_RUNNING.load(Ordering::SeqCst) {
            if last_announce
                .elapsed()
                .map(|elapsed| elapsed >= ANNOUNCE_INTERVAL)
                .unwrap_or(true)
            {
                let _ = broadcast_announcement(&socket);
                last_announce = SystemTime::now();
            }

            match socket.recv_from(&mut buffer) {
                Ok((size, source)) => {
                    if let Ok(message) = serde_json::from_slice::<DiscoveryMessage>(&buffer[..size])
                    {
                        process_message(&app, &socket, source, message);
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(_) => thread::sleep(Duration::from_millis(200)),
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub fn request_lan_connection(
    target_ip: String,
    target_device_id: String,
) -> Result<String, String> {
    let requester = local_identity()?;
    let ip = target_ip
        .trim()
        .parse::<IpAddr>()
        .map_err(|e| format!("目标 IP 无效: {e}"))?;
    let request_id = format!("lan-{}-{}", now_ms(), rand::random::<u32>());
    let message = DiscoveryMessage::ConnectRequest {
        request_id: request_id.clone(),
        target_device_id,
        requester,
    };
    let socket = UdpSocket::bind(("0.0.0.0", 0)).map_err(|e| format!("创建连接请求失败: {e}"))?;
    send_message(&socket, SocketAddr::new(ip, DISCOVERY_PORT), &message)?;
    Ok(request_id)
}

#[tauri::command]
pub fn answer_lan_connection(request_id: String, accepted: bool) -> Result<(), String> {
    let pending = PENDING_REQUESTS
        .lock()
        .unwrap()
        .remove(request_id.trim())
        .ok_or_else(|| "连接请求不存在或已失效".to_string())?;
    let responder = local_identity()?;
    let mouse_port = *LOCAL_MOUSE_PORT.lock().unwrap();

    if accepted {
        allow_runtime_discovered_device(
            pending.requester.device_id.clone(),
            pending.requester.device_name.clone(),
            pending.peer_ip,
        );
    }

    let message = DiscoveryMessage::ConnectResponse {
        request_id,
        accepted,
        responder,
        mouse_port,
        message: (!accepted).then(|| "目标设备拒绝了连接".to_string()),
    };
    let socket = UdpSocket::bind(("0.0.0.0", 0)).map_err(|e| format!("创建连接响应失败: {e}"))?;
    send_message(
        &socket,
        SocketAddr::new(pending.peer_ip, DISCOVERY_PORT),
        &message,
    )
}

#[tauri::command]
pub fn announce_device_now() -> Result<(), String> {
    let socket = UdpSocket::bind(("0.0.0.0", 0)).map_err(|e| format!("创建广播失败: {e}"))?;
    socket.set_broadcast(true).map_err(|e| e.to_string())?;
    broadcast_announcement(&socket)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn announcement_round_trip_preserves_identity_and_port() {
        let message = DiscoveryMessage::Announce {
            identity: DeviceIdentity {
                device_id: "device-12345678".to_string(),
                device_name: "Office PC".to_string(),
            },
            mouse_port: 4000,
        };
        let encoded = serde_json::to_vec(&message).unwrap();
        let decoded = serde_json::from_slice::<DiscoveryMessage>(&encoded).unwrap();
        match decoded {
            DiscoveryMessage::Announce {
                identity,
                mouse_port,
            } => {
                assert_eq!(identity.device_id, "device-12345678");
                assert_eq!(identity.device_name, "Office PC");
                assert_eq!(mouse_port, 4000);
            }
            _ => panic!("expected announcement"),
        }
    }

    #[test]
    fn connect_request_round_trip_keeps_target() {
        let message = DiscoveryMessage::ConnectRequest {
            request_id: "request-1".to_string(),
            target_device_id: "target-device".to_string(),
            requester: DeviceIdentity {
                device_id: "source-device".to_string(),
                device_name: "Source".to_string(),
            },
        };
        let encoded = serde_json::to_vec(&message).unwrap();
        let decoded = serde_json::from_slice::<DiscoveryMessage>(&encoded).unwrap();
        match decoded {
            DiscoveryMessage::ConnectRequest {
                request_id,
                target_device_id,
                requester,
            } => {
                assert_eq!(request_id, "request-1");
                assert_eq!(target_device_id, "target-device");
                assert_eq!(requester.device_id, "source-device");
            }
            _ => panic!("expected connect request"),
        }
    }
}
