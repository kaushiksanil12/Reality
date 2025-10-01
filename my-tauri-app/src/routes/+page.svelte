<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import FileDropZone from '$lib/FileDropZone.svelte';
  import DeviceList from '$lib/DeviceList.svelte';
  import TransferHistory from '$lib/TransferHistory.svelte';
  
  let selectedFiles: string[] = [];
  let connectedDevices: any[] = [];
  let transferHistory: any[] = [];
  let isServerRunning = false;
  let serverPort = 0;
  let refreshInterval: any;
  
  onMount(async () => {
    try {
      serverPort = await invoke('start_file_server');
      isServerRunning = true;
      console.log('🔒 Encrypted file server started on port:', serverPort);
      
      await invoke('start_discovery');
      console.log('🌐 Device discovery started');
      
      refreshInterval = setInterval(async () => {
        await refreshData();
      }, 3000);
      
      await refreshData();
    } catch (error) {
      console.error('Error initializing services:', error);
    }
  });
  
  async function refreshData() {
    try {
      connectedDevices = await invoke('get_devices');
      transferHistory = await invoke('get_transfers');
    } catch (error) {
      console.error('Error refreshing data:', error);
    }
  }
  
  onDestroy(async () => {
    if (refreshInterval) {
      clearInterval(refreshInterval);
    }
    try {
      await invoke('stop_discovery');
    } catch (error) {
      console.error('Error stopping services:', error);
    }
  });
  
  function handleFilesSelected(event: CustomEvent) {
    selectedFiles = event.detail;
  }
  
  async function handleSendFiles(event: CustomEvent) {
    const { files, targetDevice } = event.detail;
    
    for (const file of files) {
      try {
        await invoke('send_file', {
          filePath: file,
          targetIp: targetDevice.ip,
          targetPort: targetDevice.port
        });
        console.log('🔒 Encrypted file transfer started:', file);
      } catch (error) {
        console.error('Error sending file:', error);
      }
    }
    
    await refreshData();
  }
</script>

<svelte:head>
  <title>File Share Pro 🔒</title>
</svelte:head>

<div class="app-container">
  <header>
    <div class="header-content">
      <h1>🔒 File Share Pro</h1>
      <div class="status-indicators">
        <span class="status-badge encryption">
          🔐 Encrypted
        </span>
        <span class="status-badge" class:active={isServerRunning}>
          {#if isServerRunning}
            <span class="pulse"></span>
            Server Running (:{serverPort})
          {:else}
            Server Offline
          {/if}
        </span>
        <span class="device-count">
          💻 {connectedDevices.length} device{connectedDevices.length !== 1 ? 's' : ''}
        </span>
      </div>
    </div>
  </header>
  
  <div class="content-grid">
    <div class="file-section">
      <FileDropZone 
        bind:selectedFiles 
        {connectedDevices}
        on:filesSelected={handleFilesSelected}
        on:sendFiles={handleSendFiles}
      />
    </div>
    
    <div class="devices-section">
      <DeviceList bind:devices={connectedDevices} />
    </div>
    
    <div class="history-section">
      <TransferHistory transfers={transferHistory} />
    </div>
  </div>
</div>

<style>
  .app-container {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: #f8fafc;
  }
  
  header {
    background: linear-gradient(135deg, #2563eb 0%, #1d4ed8 100%);
    color: white;
    padding: 16px 24px;
    box-shadow: 0 2px 8px rgba(0,0,0,0.1);
  }
  
  .header-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  
  h1 {
    font-size: 24px;
    font-weight: 600;
    margin: 0;
  }
  
  .status-indicators {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  
  .status-badge {
    font-size: 13px;
    background: rgba(255, 255, 255, 0.2);
    padding: 6px 14px;
    border-radius: 20px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  
  .status-badge.encryption {
    background: rgba(16, 185, 129, 0.3);
    border: 1px solid rgba(16, 185, 129, 0.5);
  }
  
  .status-badge.active {
    background: rgba(16, 185, 129, 0.2);
  }
  
  .pulse {
    width: 8px;
    height: 8px;
    background: #10b981;
    border-radius: 50%;
    animation: pulse 2s ease-in-out infinite;
  }
  
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }
  
  .device-count {
    font-size: 14px;
    background: rgba(255, 255, 255, 0.2);
    padding: 6px 12px;
    border-radius: 20px;
  }
  
  .content-grid {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 340px 340px;
    gap: 20px;
    padding: 20px;
    overflow: auto;
  }

  @media (max-width: 1200px) {
    .content-grid {
      grid-template-columns: 1fr;
      grid-template-rows: auto auto auto;
    }
  }
</style>
