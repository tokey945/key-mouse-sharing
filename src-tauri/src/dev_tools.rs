use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::time::{Duration, Instant};

#[derive(Serialize)]
pub struct NetworkSummary {
    #[serde(rename = "primaryIp")]
    primary_ip: Option<String>,
    #[serde(rename = "loopbackIp")]
    loopback_ip: String,
    hostname: String,
}

#[derive(Serialize)]
pub struct PortCheckResult {
    port: u16,
    available: bool,
    message: String,
}

#[derive(Serialize)]
pub struct TcpProbeResult {
    reachable: bool,
    #[serde(rename = "elapsedMs")]
    elapsed_ms: u128,
    message: String,
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown-host".to_string())
}

fn primary_lan_ip() -> Option<String> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((Ipv4Addr::new(8, 8, 8, 8), 80)).ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if !ip.is_loopback() => Some(ip.to_string()),
        IpAddr::V6(ip) if !ip.is_loopback() => Some(ip.to_string()),
        _ => None,
    }
}

#[tauri::command]
pub fn get_network_summary() -> NetworkSummary {
    NetworkSummary {
        primary_ip: primary_lan_ip(),
        loopback_ip: Ipv4Addr::LOCALHOST.to_string(),
        hostname: hostname(),
    }
}

#[tauri::command]
pub fn check_port_available(port: u16) -> PortCheckResult {
    match TcpListener::bind((Ipv4Addr::UNSPECIFIED, port)) {
        Ok(listener) => {
            drop(listener);
            PortCheckResult {
                port,
                available: true,
                message: format!("端口 {} 当前可用于监听", port),
            }
        }
        Err(error) => PortCheckResult {
            port,
            available: false,
            message: format!("端口 {} 不可用: {}", port, error),
        },
    }
}

#[tauri::command]
pub fn probe_tcp_connection(
    ip: String,
    port: u16,
    timeout_ms: u64,
) -> Result<TcpProbeResult, String> {
    let ip_addr = ip
        .trim()
        .parse::<IpAddr>()
        .map_err(|error| format!("IP 地址解析失败: {}", error))?;
    let timeout = Duration::from_millis(timeout_ms.clamp(200, 10_000));
    let start = Instant::now();

    match TcpStream::connect_timeout(&SocketAddr::new(ip_addr, port), timeout) {
        Ok(stream) => {
            drop(stream);
            Ok(TcpProbeResult {
                reachable: true,
                elapsed_ms: start.elapsed().as_millis(),
                message: format!("{}:{} 可连接", ip_addr, port),
            })
        }
        Err(error) => Ok(TcpProbeResult {
            reachable: false,
            elapsed_ms: start.elapsed().as_millis(),
            message: format!("{}:{} 不可连接: {}", ip_addr, port, error),
        }),
    }
}
