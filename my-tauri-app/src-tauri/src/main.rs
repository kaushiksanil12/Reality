// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// use tauri::Manager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::State;
use uuid::Uuid;
use mdns_sd::{ServiceDaemon, ServiceInfo, ServiceEvent};
// use std::time::Duration;

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
}

// App state
struct AppState {
    devices: Arc<Mutex<HashMap<String, Device>>>,
    transfers: Arc<Mutex<Vec<FileTransfer>>>,
    mdns_daemon: Arc<Mutex<Option<ServiceDaemon>>>,
    device_id: String,
    device_name: String,
    server_port: u16,
    encryption_key: [u8; 32],
}

// Generate encryption key (shared across all devices for simplicity)
// In production, use proper key exchange protocol
fn generate_encryption_key() -> [u8; 32] {
    // For demo purposes, using a fixed key so all instances can communicate
    // In production, implement proper key exchange (Diffie-Hellman, etc.)
    let fixed_key = b"FileShareProSecureKey12345678!!8"; // Exactly 32 bytes
    *fixed_key
}

// Encrypt data
fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    
    // Generate random nonce
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    // Encrypt
    let ciphertext = cipher.encrypt(nonce, data)
        .map_err(|e| format!("Encryption error: {:?}", e))?;
    
    // Prepend nonce to ciphertext
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    
    Ok(result)
}

// Decrypt data
fn decrypt_data(encrypted_data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    if encrypted_data.len() < 12 {
        return Err("Invalid encrypted data".to_string());
    }
    
    // Extract nonce and ciphertext
    let nonce = Nonce::from_slice(&encrypted_data[..12]);
    let ciphertext = &encrypted_data[12..];
    
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    
    // Decrypt
    cipher.decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption error: {:?}", e))
}

// Initialize mDNS service discovery
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
                    
                    // Don't add ourselves to the device list
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
    
    Ok("Discovery started with encryption enabled 🔒".to_string())
}

// Get discovered devices
#[tauri::command]
fn get_devices(state: State<'_, AppState>) -> Result<Vec<Device>, String> {
    let devices = state.devices.lock().unwrap();
    Ok(devices.values().cloned().collect())
}

// Start file receiver server
#[tauri::command]
async fn start_file_server(state: State<'_, AppState>) -> Result<u16, String> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", state.server_port))
        .map_err(|e| e.to_string())?;
    
    let port = listener.local_addr()
        .map_err(|e| e.to_string())?
        .port();
    
    let transfers = state.transfers.clone();
    let encryption_key = state.encryption_key;
    
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let transfers = transfers.clone();
                    thread::spawn(move || {
                        if let Err(e) = handle_incoming_file(stream, transfers, encryption_key) {
                            eprintln!("Error handling file: {}", e);
                        }
                    });
                }
                Err(e) => eprintln!("Connection error: {}", e),
            }
        }
    });
    
    Ok(port)
}

// Handle incoming encrypted file transfer
fn handle_incoming_file(
    mut stream: TcpStream,
    transfers: Arc<Mutex<Vec<FileTransfer>>>,
    encryption_key: [u8; 32],
) -> std::io::Result<()> {
    // Read filename length
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let filename_len = u32::from_be_bytes(len_buf) as usize;
    
    // Read filename
    let mut filename_buf = vec![0u8; filename_len];
    stream.read_exact(&mut filename_buf)?;
    let filename = String::from_utf8_lossy(&filename_buf).to_string();
    
    // Read file size
    let mut size_buf = [0u8; 8];
    stream.read_exact(&mut size_buf)?;
    let file_size = u64::from_be_bytes(size_buf);
    
    // Create transfer record
    let transfer_id = Uuid::new_v4().to_string();
    let transfer = FileTransfer {
        id: transfer_id.clone(),
        filename: filename.clone(),
        size: file_size,
        progress: 0,
        status: "Receiving 🔒".to_string(),
        from_device: "Remote".to_string(),
        to_device: "This Device".to_string(),
        encrypted: true,
    };
    
    {
        let mut transfers = transfers.lock().unwrap();
        transfers.push(transfer.clone());
    }
    
    let download_path = dirs::download_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap())
        .join(&filename);
    
    // Receive encrypted file
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
        
        // Update progress
        let mut transfers = transfers.lock().unwrap();
        if let Some(t) = transfers.iter_mut().find(|t| t.id == transfer_id) {
            t.progress = received;
        }
    }
    
    // Decrypt file
    match decrypt_data(&encrypted_data, &encryption_key) {
        Ok(decrypted_data) => {
            std::fs::write(&download_path, decrypted_data)?;
            
            // Update status
            let mut transfers = transfers.lock().unwrap();
            if let Some(t) = transfers.iter_mut().find(|t| t.id == transfer_id) {
                t.status = "Completed ✅ (Decrypted)".to_string();
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

// Send encrypted file to device
#[tauri::command]
async fn send_file(
    file_path: String,
    target_ip: String,
    target_port: u16,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let transfers = state.transfers.clone();
    let encryption_key = state.encryption_key;
    
    thread::spawn(move || {
        if let Err(e) = send_file_internal(file_path, target_ip, target_port, transfers, encryption_key) {
            eprintln!("Error sending file: {}", e);
        }
    });
    
    Ok("Encrypted transfer started 🔒".to_string())
}

fn send_file_internal(
    file_path: String,
    target_ip: String,
    target_port: u16,
    transfers: Arc<Mutex<Vec<FileTransfer>>>,
    encryption_key: [u8; 32],
) -> std::io::Result<()> {
    let mut stream = TcpStream::connect(format!("{}:{}", target_ip, target_port))?;
    
    // Read file
    let file_data = std::fs::read(&file_path)?;
    
    let filename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    
    // Encrypt file
    let encrypted_data = encrypt_data(&file_data, &encryption_key)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    
    let encrypted_size = encrypted_data.len() as u64;
    
    // Create transfer record
    let transfer_id = Uuid::new_v4().to_string();
    let transfer = FileTransfer {
        id: transfer_id.clone(),
        filename: filename.to_string(),
        size: encrypted_size,
        progress: 0,
        status: "Encrypting & Sending 🔒".to_string(),
        from_device: "This Device".to_string(),
        to_device: target_ip.clone(),
        encrypted: true,
    };
    
    {
        let mut transfers = transfers.lock().unwrap();
        transfers.push(transfer.clone());
    }
    
    // Send filename length
    let filename_bytes = filename.as_bytes();
    stream.write_all(&(filename_bytes.len() as u32).to_be_bytes())?;
    
    // Send filename
    stream.write_all(filename_bytes)?;
    
    // Send encrypted file size
    stream.write_all(&encrypted_size.to_be_bytes())?;
    
    // Send encrypted content
    let mut sent = 0u64;
    let chunk_size = 8192;
    
    for chunk in encrypted_data.chunks(chunk_size) {
        stream.write_all(chunk)?;
        sent += chunk.len() as u64;
        
        // Update progress
        let mut transfers = transfers.lock().unwrap();
        if let Some(t) = transfers.iter_mut().find(|t| t.id == transfer_id) {
            t.progress = sent;
            if sent >= encrypted_size {
                t.status = "Completed ✅ (Encrypted)".to_string();
            }
        }
    }
    
    Ok(())
}

// Get transfer history
#[tauri::command]
fn get_transfers(state: State<'_, AppState>) -> Result<Vec<FileTransfer>, String> {
    let transfers = state.transfers.lock().unwrap();
    Ok(transfers.clone())
}

// Stop discovery
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
    
    // Use fixed key so all devices can communicate
    let encryption_key = generate_encryption_key();
    
    println!("🔐 Encryption enabled - ChaCha20-Poly1305");
    println!("🔑 Using shared encryption key");
    
    let app_state = AppState {
        devices: Arc::new(Mutex::new(HashMap::new())),
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
            start_file_server,
            send_file,
            get_transfers,
            stop_discovery,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
