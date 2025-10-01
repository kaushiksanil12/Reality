<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    
    export let devices: any[] = [];
    let isScanning = false;
    
    async function scanForDevices() {
      isScanning = true;
      try {
        await invoke('start_discovery');
        setTimeout(async () => {
          devices = await invoke('get_devices');
          isScanning = false;
        }, 2000);
      } catch (error) {
        console.error('Error scanning for devices:', error);
        isScanning = false;
      }
    }
    
    function getDeviceIcon(type: string): string {
      const icons: Record<string, string> = {
        laptop: '💻',
        phone: '📱',
        desktop: '🖥️',
        tablet: '📱'
      };
      return icons[type] || '💻';
    }
    
    function getStatusColor(status: string): string {
      const colors: Record<string, string> = {
        Available: '#10b981',
        Busy: '#f59e0b',
        Offline: '#6b7280'
      };
      return colors[status] || '#6b7280';
    }
  </script>
  
  <div class="panel">
    <div class="panel-header">
      <h3>🌐 Available Devices</h3>
      <button class="scan-btn" on:click={scanForDevices} disabled={isScanning}>
        {#if isScanning}
          <span class="spinner-icon">🔄</span> Scanning...
        {:else}
          <span>🔍</span> Scan
        {/if}
      </button>
    </div>
    
    {#if isScanning}
      <div class="scanning-indicator">
        <div class="spinner"></div>
        <p>Scanning for devices...</p>
        <span class="scan-hint">Looking for encrypted file sharing devices</span>
      </div>
    {:else if devices.length === 0}
      <div class="no-devices">
        <div class="empty-icon">📡</div>
        <p>No devices found</p>
        <span class="empty-hint">Make sure other devices are running the app on the same network</span>
        <button class="scan-btn-large" on:click={scanForDevices}>
          Scan Again
        </button>
      </div>
    {:else}
      <div class="device-list">
        {#each devices as device (device.id)}
          <div class="device-item" class:busy={device.status === 'Busy'}>
            <div class="device-icon-wrapper">
              <div class="device-icon">{getDeviceIcon(device.device_type)}</div>
              <div class="status-dot" style="background-color: {getStatusColor(device.status)}"></div>
            </div>
            
            <div class="device-info">
              <span class="device-name">{device.name}</span>
              <div class="device-meta">
                <span class="device-status" style="color: {getStatusColor(device.status)}">
                  {device.status}
                </span>
                <span class="device-time">• {device.ip}</span>
              </div>
            </div>
            
            <span class="encrypted-badge">🔐</span>
          </div>
        {/each}
      </div>
      
      <div class="device-footer">
        <span class="device-count">
          {devices.filter(d => d.status === 'Available').length} of {devices.length} available
        </span>
      </div>
    {/if}
  </div>
  
  <style>
    .panel {
      background: white;
      border-radius: 12px;
      padding: 24px;
      box-shadow: 0 1px 3px rgba(0,0,0,0.1);
      height: fit-content;
    }
    
    .panel-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 20px;
    }
    
    .panel-header h3 {
      margin: 0;
      font-size: 18px;
      color: #1e293b;
      font-weight: 600;
    }
    
    .scan-btn {
      background: #3b82f6;
      color: white;
      border: none;
      padding: 8px 12px;
      border-radius: 6px;
      cursor: pointer;
      font-size: 12px;
      font-weight: 500;
      transition: background 0.2s;
      display: flex;
      align-items: center;
      gap: 6px;
    }
    
    .scan-btn:hover:not(:disabled) {
      background: #2563eb;
    }
    
    .scan-btn:disabled {
      background: #9ca3af;
      cursor: not-allowed;
    }
    
    .spinner-icon {
      display: inline-block;
      animation: spin 1s linear infinite;
    }
    
    @keyframes spin {
      0% { transform: rotate(0deg); }
      100% { transform: rotate(360deg); }
    }
    
    .scanning-indicator {
      text-align: center;
      padding: 40px 20px;
    }
    
    .spinner {
      width: 32px;
      height: 32px;
      border: 3px solid #e5e7eb;
      border-top: 3px solid #3b82f6;
      border-radius: 50%;
      margin: 0 auto 16px;
      animation: spin 1s linear infinite;
    }
    
    .scanning-indicator p {
      margin: 0 0 8px 0;
      color: #1e293b;
      font-weight: 500;
    }
    
    .scan-hint {
      font-size: 12px;
      color: #64748b;
    }
    
    .no-devices {
      text-align: center;
      padding: 40px 20px;
      color: #64748b;
    }
    
    .empty-icon {
      font-size: 48px;
      margin-bottom: 16px;
    }
    
    .no-devices p {
      margin: 0 0 8px 0;
      font-size: 16px;
      color: #1e293b;
      font-weight: 500;
    }
    
    .empty-hint {
      display: block;
      font-size: 12px;
      color: #64748b;
      margin-bottom: 20px;
    }
    
    .scan-btn-large {
      background: #3b82f6;
      color: white;
      border: none;
      padding: 10px 20px;
      border-radius: 8px;
      cursor: pointer;
      font-size: 14px;
      font-weight: 500;
    }
    
    .scan-btn-large:hover {
      background: #2563eb;
    }
    
    .device-list {
      display: flex;
      flex-direction: column;
      gap: 8px;
    }
    
    .device-item {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 16px;
      border: 1px solid #e2e8f0;
      border-radius: 8px;
      transition: all 0.2s;
      background: white;
    }
    
    .device-item:hover {
      border-color: #10b981;
      box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    }
    
    .device-item.busy {
      opacity: 0.7;
      background: #f9fafb;
    }
    
    .device-icon-wrapper {
      position: relative;
      flex-shrink: 0;
    }
    
    .device-icon {
      font-size: 24px;
    }
    
    .status-dot {
      position: absolute;
      bottom: -2px;
      right: -2px;
      width: 10px;
      height: 10px;
      border-radius: 50%;
      border: 2px solid white;
    }
    
    .device-info {
      flex: 1;
      display: flex;
      flex-direction: column;
      gap: 4px;
      min-width: 0;
    }
    
    .device-name {
      font-weight: 500;
      font-size: 14px;
      color: #1e293b;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    
    .device-meta {
      display: flex;
      align-items: center;
      gap: 4px;
      font-size: 11px;
    }
    
    .device-status {
      font-weight: 500;
    }
    
    .device-time {
      color: #94a3b8;
    }
    
    .encrypted-badge {
      font-size: 16px;
      flex-shrink: 0;
    }
    
    .device-footer {
      margin-top: 16px;
      padding-top: 16px;
      border-top: 1px solid #e2e8f0;
      text-align: center;
    }
    
    .device-count {
      font-size: 12px;
      color: #64748b;
      font-weight: 500;
    }
  </style>
  