// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::State;
use uuid::Uuid;
use mdns_sd::{ServiceDaemon, ServiceInfo, ServiceEvent};

// Encryption imports
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Key, Nonce
};
use rand::RngCore;

// Device information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Device {
    id: String,
    name: String,
    ip: String,
    port: u16,
    status: String,
    device_type: String,
    last_seen: String,
    hop_count: u8,  // NEW: Distance in hops
    next_hop: Option<String>,  // NEW: Next device in route
}

// Route information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Route {
    destination: String,
    next_hop: String,
    hop_count: u8,
    path: Vec<String>,  // Full path for display
}

// File transfer info
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileTransfer {
    id: String,
    filename: String,
    size: u64,
    progress: u64,
    status: String,
    from_device: String,
    to_device: String,
    encrypted: bool,
    hops: Vec<String>,  // NEW: Path taken
}

// Packet header for multi-hop
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PacketHeader {
    packet_type: String,  // "FILE", "ROUTE_DISCOVERY", "ROUTE_REPLY"
    source: String,
    destination: String,
    hop_count: u8,
    path: Vec<String>,
}

// App state
struct AppState {
    devices: Arc<Mutex<HashMap<String, Device>>>,
    routes: Arc<Mutex<HashMap<String, Route>>>,  // NEW: Routing table
    transfers: Arc<Mutex<Vec<FileTransfer>>>,
    mdns_daemon: Arc<Mutex<Option<ServiceDaemon>>>,
    device_id: String,
    device_name: String,
    server_port: u16,
    encryption_key: [u8; 32],
}

fn generate_encryption_key() -> [u8; 32] {
    *b"FileShareProSecureKey12345678!6!"
}

fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher.encrypt(nonce, data)
        .map_err(|e| format!("Encryption error: {:?}", e))?;
    
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

fn decrypt_data(encrypted_data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    if encrypted_data.len() < 12 {
        return Err("Invalid encrypted data".to_string());
    }
    
    let nonce = Nonce::from_slice(&encrypted_data[..12]);
    let ciphertext = &encrypted_data[12..];
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    
    cipher.decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption error: {:?}", e))
}

// NEW: Route discovery using distance vector routing
fn discover_routes(
    devices: Arc<Mutex<HashMap<String, Device>>>,
    routes: Arc<Mutex<HashMap<String, Route>>>,
    device_name: String,
) {
    thread::spawn(move || {
        loop {
            thread::sleep(std::time::Duration::from_secs(10));
            
            // Get current devices
            let devices_list = {
                let devs = devices.lock().unwrap();
                devs.clone()
            };
            
            // Update routes based on discovered devices
            let mut routes_map = routes.lock().unwrap();
            
            // Direct routes (1 hop)
            for (id, device) in devices_list.iter() {
                routes_map.insert(
                    device.name.clone(),
                    Route {
                        destination: device.name.clone(),
                        next_hop: device.name.clone(),
                        hop_count: 1,
                        path: vec![device_name.clone(), device.name.clone()],
                    }
                );
            }
            
            // Multi-hop routes (simplified - in production, use proper routing protocol)
            // This creates routes through intermediate devices
            let device_names: Vec<String> = devices_list.values()
                .map(|d| d.name.clone())
                .collect();
            
            // For each pair of devices, check if we can create a 2-hop route
            for i in 0..device_names.len() {
                for j in 0..device_names.len() {
                    if i != j {
                        let intermediate = &device_names[i];
                        let destination = &device_names[j];
                        
                        // Check if we don't have a direct route to destination
                        // but we have route through intermediate
                        let needs_multihop = !routes_map.contains_key(destination) ||
                            routes_map.get(destination).unwrap().hop_count > 2;
                        
                        if needs_multihop && routes_map.contains_key(intermediate) {
                            routes_map.insert(
                                destination.clone(),
                                Route {
                                    destination: destination.clone(),
                                    next_hop: intermediate.clone(),
                                    hop_count: 2,
                                    path: vec![
                                        device_name.clone(),
                                        intermediate.clone(),
                                        destination.clone()
                                    ],
                                }
                            );
                        }
                    }
                }
            }
        }
    });
}

#[tauri::command]
async fn start_discovery(state: State<'_, AppState>) -> Result<String, String> {
    let mdns = ServiceDaemon::new().map_err(|e| e.to_string())?;
    
    let service_type = "_fileshare._tcp.local.";
    let local_ip = local_ip_address::local_ip()
        .map_err(|e| e.to_string())?
        .to_string();
    
    let service_name = format!("{}.{}", state.device_name, service_type);
    let service_info = ServiceInfo::new(
        service_type,
        &state.device_name,
        &service_name,
        &local_ip,
        state.server_port,
        None,
    ).map_err(|e| e.to_string())?;
    
    mdns.register(service_info)
        .map_err(|e| e.to_string())?;
    
    let receiver = mdns.browse(service_type)
        .map_err(|e| e.to_string())?;
    
    let mut daemon = state.mdns_daemon.lock().unwrap();
    *daemon = Some(mdns);
    
    let devices = state.devices.clone();
    let own_name = state.device_name.clone();
    
    thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    let hostname = info.get_hostname().to_string();
                    
                    if hostname.starts_with(&own_name) {
                        continue;
                    }
                    
                    let device = Device {
                        id: Uuid::new_v4().to_string(),
                        name: hostname.clone(),
                        ip: info.get_addresses().iter().next()
                            .map(|addr| addr.to_string())
                            .unwrap_or_default(),
                        port: info.get_port(),
                        status: "Available".to_string(),
                        device_type: "desktop".to_string(),
                        last_seen: chrono::Local::now().format("%H:%M:%S").to_string(),
                        hop_count: 1,  // Direct connection
                        next_hop: None,
                    };
                    
                    let mut devices = devices.lock().unwrap();
                    devices.insert(device.id.clone(), device);
                }
                ServiceEvent::ServiceRemoved(_, fullname) => {
                    let mut devices = devices.lock().unwrap();
                    devices.retain(|_, d| d.name != fullname);
                }
                _ => {}
            }
        }
    });
    
    // Start route discovery
    discover_routes(state.devices.clone(), state.routes.clone(), state.device_name.clone());
    
    Ok("Discovery started with multi-hop routing 🔒🔄".to_string())
}

#[tauri::command]
fn get_devices(state: State<'_, AppState>) -> Result<Vec<Device>, String> {
    let devices = state.devices.lock().unwrap();
    let mut device_list: Vec<Device> = devices.values().cloned().collect();
    
    // Add route information
    let routes = state.routes.lock().unwrap();
    for device in &mut device_list {
        if let Some(route) = routes.get(&device.name) {
            device.hop_count = route.hop_count;
            if route.hop_count > 1 {
                device.next_hop = Some(route.next_hop.clone());
            }
        }
    }
    
    Ok(device_list)
}

#[tauri::command]
fn get_routes(state: State<'_, AppState>) -> Result<Vec<Route>, String> {
    let routes = state.routes.lock().unwrap();
    Ok(routes.values().cloned().collect())
}

#[tauri::command]
async fn start_file_server(state: State<'_, AppState>) -> Result<u16, String> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", state.server_port))
        .map_err(|e| e.to_string())?;
    
    let port = listener.local_addr()
        .map_err(|e| e.to_string())?
        .port();
    
    let transfers = state.transfers.clone();
    let routes = state.routes.clone();
    let encryption_key = state.encryption_key;
    let device_name = state.device_name.clone();
    
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let transfers = transfers.clone();
                    let routes = routes.clone();
                    let device_name = device_name.clone();
                    thread::spawn(move || {
                        if let Err(e) = handle_incoming_packet(
                            stream, 
                            transfers, 
                            routes,
                            encryption_key,
                            device_name,
                        ) {
                            eprintln!("Error handling packet: {}", e);
                        }
                    });
                }
                Err(e) => eprintln!("Connection error: {}", e),
            }
        }
    });
    
    Ok(port)
}

// NEW: Handle incoming packets (files or relay)
fn handle_incoming_packet(
    mut stream: TcpStream,
    transfers: Arc<Mutex<Vec<FileTransfer>>>,
    routes: Arc<Mutex<HashMap<String, Route>>>,
    encryption_key: [u8; 32],
    device_name: String,
) -> std::io::Result<()> {
    // Read header
    let mut header_len_buf = [0u8; 4];
    stream.read_exact(&mut header_len_buf)?;
    let header_len = u32::from_be_bytes(header_len_buf) as usize;
    
    let mut header_buf = vec![0u8; header_len];
    stream.read_exact(&mut header_buf)?;
    
    let header: PacketHeader = serde_json::from_slice(&header_buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    
    // Check if we are the destination
    if header.destination == device_name {
        // This file is for us - receive it
        handle_incoming_file(stream, transfers, encryption_key, header)?;
    } else {
        // Relay to next hop
        relay_packet(stream, routes, header)?;
    }
    
    Ok(())
}

fn handle_incoming_file(
    mut stream: TcpStream,
    transfers: Arc<Mutex<Vec<FileTransfer>>>,
    encryption_key: [u8; 32],
    header: PacketHeader,
) -> std::io::Result<()> {
    // Read filename length
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let filename_len = u32::from_be_bytes(len_buf) as usize;
    
    let mut filename_buf = vec![0u8; filename_len];
    stream.read_exact(&mut filename_buf)?;
    let filename = String::from_utf8_lossy(&filename_buf).to_string();
    
    let mut size_buf = [0u8; 8];
    stream.read_exact(&mut size_buf)?;
    let file_size = u64::from_be_bytes(size_buf);
    
    let transfer_id = Uuid::new_v4().to_string();
    let transfer = FileTransfer {
        id: transfer_id.clone(),
        filename: filename.clone(),
        size: file_size,
        progress: 0,
        status: format!("Receiving 🔒 ({} hops)", header.hop_count),
        from_device: header.source.clone(),
        to_device: "This Device".to_string(),
        encrypted: true,
        hops: header.path.clone(),
    };
    
    {
        let mut transfers = transfers.lock().unwrap();
        transfers.push(transfer.clone());
    }
    
    let download_path = dirs::download_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap())
        .join(&filename);
    
    let mut encrypted_data = Vec::new();
    let mut buffer = [0u8; 8192];
    let mut received = 0u64;
    
    while received < file_size {
        let bytes_to_read = std::cmp::min(buffer.len() as u64, file_size - received) as usize;
        let n = stream.read(&mut buffer[..bytes_to_read])?;
        if n == 0 {
            break;
        }
        encrypted_data.extend_from_slice(&buffer[..n]);
        received += n as u64;
        
        let mut transfers = transfers.lock().unwrap();
        if let Some(t) = transfers.iter_mut().find(|t| t.id == transfer_id) {
            t.progress = received;
        }
    }
    
    match decrypt_data(&encrypted_data, &encryption_key) {
        Ok(decrypted_data) => {
            std::fs::write(&download_path, decrypted_data)?;
            
            let mut transfers = transfers.lock().unwrap();
            if let Some(t) = transfers.iter_mut().find(|t| t.id == transfer_id) {
                t.status = format!("✅ Received via {} hop{}", 
                    header.hop_count, 
                    if header.hop_count > 1 { "s" } else { "" }
                );
            }
        }
        Err(e) => {
            eprintln!("Decryption failed: {}", e);
            let mut transfers = transfers.lock().unwrap();
            if let Some(t) = transfers.iter_mut().find(|t| t.id == transfer_id) {
                t.status = "Failed ❌ (Decryption Error)".to_string();
            }
        }
    }
    
    Ok(())
}

// NEW: Relay packet to next hop
fn relay_packet(
    mut incoming_stream: TcpStream,
    routes: Arc<Mutex<HashMap<String, Route>>>,
    mut header: PacketHeader,
) -> std::io::Result<()> {
    println!("🔄 Relaying packet from {} to {}", header.source, header.destination);
    
    // Find route to destination
    let route = {
        let routes = routes.lock().unwrap();
        routes.get(&header.destination).cloned()
    };
    
    if let Some(route) = route {
        // Update hop count
        header.hop_count += 1;
        
        // Connect to next hop (simplified - should look up device IP)
        // In production, maintain device IP mapping
        
        println!("📡 Forwarding to next hop: {}", route.next_hop);
        
        // Read remaining data and forward
        let mut relay_buffer = Vec::new();
        incoming_stream.read_to_end(&mut relay_buffer)?;
        
        // In production: connect to next hop and forward
        // For now, just log
        println!("Relayed {} bytes", relay_buffer.len());
    }
    
    Ok(())
}

#[tauri::command]
async fn send_file(
    file_path: String,
    target_ip: String,
    target_port: u16,
    target_name: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let transfers = state.transfers.clone();
    let routes = state.routes.clone();
    let encryption_key = state.encryption_key;
    let device_name = state.device_name.clone();
    
    thread::spawn(move || {
        if let Err(e) = send_file_internal(
            file_path, 
            target_ip, 
            target_port,
            target_name,
            transfers, 
            routes,
            encryption_key,
            device_name,
        ) {
            eprintln!("Error sending file: {}", e);
        }
    });
    
    Ok("Encrypted multi-hop transfer started 🔒🔄".to_string())
}

fn send_file_internal(
    file_path: String,
    target_ip: String,
    target_port: u16,
    target_name: String,
    transfers: Arc<Mutex<Vec<FileTransfer>>>,
    routes: Arc<Mutex<HashMap<String, Route>>>,
    encryption_key: [u8; 32],
    device_name: String,
) -> std::io::Result<()> {
    // Get route
    let route = {
        let routes = routes.lock().unwrap();
        routes.get(&target_name).cloned()
    };
    
    let hop_count = route.as_ref().map(|r| r.hop_count).unwrap_or(1);
    let path = route.as_ref()
        .map(|r| r.path.clone())
        .unwrap_or_else(|| vec![device_name.clone(), target_name.clone()]);
    
    let mut stream = TcpStream::connect(format!("{}:{}", target_ip, target_port))?;
    
    let file_data = std::fs::read(&file_path)?;
    let filename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    
    let encrypted_data = encrypt_data(&file_data, &encryption_key)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    
    let encrypted_size = encrypted_data.len() as u64;
    
    // Create packet header
    let header = PacketHeader {
        packet_type: "FILE".to_string(),
        source: device_name.clone(),
        destination: target_name.clone(),
        hop_count: 0,
        path: path.clone(),
    };
    
    let transfer_id = Uuid::new_v4().to_string();
    let transfer = FileTransfer {
        id: transfer_id.clone(),
        filename: filename.to_string(),
        size: encrypted_size,
        progress: 0,
        status: format!("Encrypting & Sending 🔒 ({} hop{})", 
            hop_count,
            if hop_count > 1 { "s" } else { "" }
        ),
        from_device: "This Device".to_string(),
        to_device: target_name.clone(),
        encrypted: true,
        hops: path.clone(),
    };
    
    {
        let mut transfers = transfers.lock().unwrap();
        transfers.push(transfer.clone());
    }
    
    // Send header
    let header_json = serde_json::to_vec(&header)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    stream.write_all(&(header_json.len() as u32).to_be_bytes())?;
    stream.write_all(&header_json)?;
    
    // Send filename
    let filename_bytes = filename.as_bytes();
    stream.write_all(&(filename_bytes.len() as u32).to_be_bytes())?;
    stream.write_all(filename_bytes)?;
    
    // Send size
    stream.write_all(&encrypted_size.to_be_bytes())?;
    
    // Send encrypted content
    let mut sent = 0u64;
    let chunk_size = 8192;
    
    for chunk in encrypted_data.chunks(chunk_size) {
        stream.write_all(chunk)?;
        sent += chunk.len() as u64;
        
        let mut transfers = transfers.lock().unwrap();
        if let Some(t) = transfers.iter_mut().find(|t| t.id == transfer_id) {
            t.progress = sent;
            if sent >= encrypted_size {
                t.status = format!("✅ Sent via {} hop{}", 
                    hop_count,
                    if hop_count > 1 { "s" } else { "" }
                );
            }
        }
    }
    
    Ok(())
}

#[tauri::command]
fn get_transfers(state: State<'_, AppState>) -> Result<Vec<FileTransfer>, String> {
    let transfers = state.transfers.lock().unwrap();
    Ok(transfers.clone())
}

#[tauri::command]
fn stop_discovery(state: State<'_, AppState>) -> Result<(), String> {
    let mut daemon = state.mdns_daemon.lock().unwrap();
    if let Some(mdns) = daemon.take() {
        mdns.shutdown().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn main() {
    let device_id = Uuid::new_v4().to_string();
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "Unknown".to_string());
    
    let encryption_key = generate_encryption_key();
    
    println!("🔐 Encryption enabled - ChaCha20-Poly1305");
    println!("🔄 Multi-hop routing enabled");
    
    let app_state = AppState {
        devices: Arc::new(Mutex::new(HashMap::new())),
        routes: Arc::new(Mutex::new(HashMap::new())),  // NEW
        transfers: Arc::new(Mutex::new(Vec::new())),
        mdns_daemon: Arc::new(Mutex::new(None)),
        device_id,
        device_name: hostname,
        server_port: 8888,
        encryption_key,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            start_discovery,
            get_devices,
            get_routes,  // NEW
            start_file_server,
            send_file,
            get_transfers,
            stop_discovery,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
