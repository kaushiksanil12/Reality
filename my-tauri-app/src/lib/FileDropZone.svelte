<script lang="ts">
    import { createEventDispatcher } from 'svelte';
    import { open } from '@tauri-apps/plugin-dialog';
    
    export let selectedFiles: string[] = [];
    export let connectedDevices: any[] = [];
    
    const dispatch = createEventDispatcher();
    let isDragOver = false;
    let selectedDevice: any = null;
    let showDeviceSelector = false;
    
    async function selectFiles() {
      try {
        const result = await open({
          multiple: true,
          title: 'Select files to share (will be encrypted)',
        });
        
        if (result) {
          const files = Array.isArray(result) ? result : [result];
          selectedFiles = [...selectedFiles, ...files];
          dispatch('filesSelected', selectedFiles);
        }
      } catch (error) {
        console.error('Error selecting files:', error);
      }
    }
    
    function handleDragOver(event: DragEvent) {
      event.preventDefault();
      event.stopPropagation();
      isDragOver = true;
    }
    
    function handleDragLeave(event: DragEvent) {
      event.preventDefault();
      event.stopPropagation();
      isDragOver = false;
    }
    
    function handleDrop(event: DragEvent) {
      event.preventDefault();
      event.stopPropagation();
      isDragOver = false;
    }
    
    function removeFile(index: number) {
      selectedFiles = selectedFiles.filter((_, i) => i !== index);
    }
    
    function clearAll() {
      selectedFiles = [];
    }
    
    function getFileName(path: string): string {
      return path.split(/[\\/]/).pop() || path;
    }
    
    function getFileIcon(filename: string): string {
      const ext = filename.split('.').pop()?.toLowerCase();
      const iconMap: Record<string, string> = {
        'pdf': '📕', 'doc': '📘', 'docx': '📘',
        'xls': '📊', 'xlsx': '📊', 'ppt': '📙', 'pptx': '📙',
        'jpg': '🖼️', 'jpeg': '🖼️', 'png': '🖼️', 'gif': '🖼️',
        'mp4': '🎬', 'mov': '🎬', 'avi': '🎬',
        'mp3': '🎵', 'wav': '🎵',
        'zip': '📦', 'rar': '📦',
        'txt': '📝', 'md': '📝',
      };
      return iconMap[ext || ''] || '📄';
    }
    
    function initiateShare() {
      if (selectedFiles.length === 0) return;
      
      if (connectedDevices.length === 0) {
        alert('No devices available. Please wait for device discovery.');
        return;
      }
      
      showDeviceSelector = true;
    }
    
    function sendToDevice(device: any) {
      dispatch('sendFiles', {
        files: selectedFiles,
        targetDevice: device
      });
      
      showDeviceSelector = false;
      selectedFiles = [];
      selectedDevice = null;
    }
    
    function cancelShare() {
      showDeviceSelector = false;
      selectedDevice = null;
    }
  </script>
  
  <div class="panel">
    <div class="panel-header">
      <h3>📁 Select Files</h3>
      <span class="encryption-badge">🔐 End-to-End Encrypted</span>
    </div>
    
    <div class="drop-zone" 
         class:drag-over={isDragOver}
         on:dragover={handleDragOver}
         on:dragleave={handleDragLeave}
         on:drop={handleDrop}
         role="button"
         tabindex="0"
         on:click={selectFiles}
         on:keydown={(e) => e.key === 'Enter' && selectFiles()}>
      
      <div class="drop-content">
        <div class="drop-icon">🔒</div>
        <h4>Click to select files</h4>
        <p class="drop-hint">Files will be encrypted before transfer</p>
        <button class="select-btn" on:click|stopPropagation={selectFiles}>
          Browse Files
        </button>
      </div>
    </div>
    
    {#if selectedFiles.length > 0}
      <div class="file-list">
        <div class="list-header">
          <h4>Selected Files ({selectedFiles.length})</h4>
          <button class="clear-all-btn" on:click={clearAll}>
            Clear All
          </button>
        </div>
        
        <div class="files-container">
          {#each selectedFiles as file, index}
            <div class="file-item">
              <div class="file-icon">{getFileIcon(getFileName(file))}</div>
              <div class="file-info">
                <span class="file-name" title={file}>{getFileName(file)}</span>
                <span class="file-path">{file}</span>
              </div>
              <button class="remove-btn" on:click|stopPropagation={() => removeFile(index)} title="Remove file">
                ✕
              </button>
            </div>
          {/each}
        </div>
        
        <div class="action-buttons">
          <button class="share-btn" on:click={initiateShare} disabled={selectedFiles.length === 0 || connectedDevices.length === 0}>
            <span>🔒</span>
            Encrypt & Share {selectedFiles.length} file{selectedFiles.length !== 1 ? 's' : ''}
          </button>
        </div>
      </div>
    {/if}
  </div>
  
  {#if showDeviceSelector}
  <div class="modal-overlay">
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="modal-content" on:click|stopPropagation role="dialog" aria-modal="true">
      <h3 id="modal-title">Select Target Device</h3>
      <p class="modal-hint" id="modal-desc">
        Files will be encrypted end-to-end. Choose which device to send {selectedFiles.length} file{selectedFiles.length !== 1 ? 's' : ''} to:
      </p>

      <div class="device-selector-list">
        {#each connectedDevices as device}
          {#if device.status === 'Available'}
            <button class="device-selector-item" on:click={() => sendToDevice(device)}>
              <div class="device-icon">💻</div>
              <div class="device-details">
                <span class="device-name">{device.name}</span>
                <span class="device-ip">{device.ip}:{device.port}</span>
              </div>
              <span class="arrow">→</span>
            </button>
          {/if}
        {/each}
      </div>

      <button 
        class="modal-overlay-button" 
        on:click={cancelShare}
        aria-label="Close device selector (press Escape)"
      >
        Close
      </button>
    </div>
  </div>
{/if}



  
  <style>
    .panel {
      background: white;
      border-radius: 12px;
      padding: 24px;
      box-shadow: 0 1px 3px rgba(0,0,0,0.1);
    }
    
    .panel-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 20px;
    }
    
    .panel h3 {
      margin: 0;
      font-size: 18px;
      color: #1e293b;
      font-weight: 600;
    }
    
    .encryption-badge {
      background: linear-gradient(135deg, #10b981, #059669);
      color: white;
      padding: 4px 12px;
      border-radius: 12px;
      font-size: 11px;
      font-weight: 600;
    }
  
    .drop-zone {
      border: 2px dashed #cbd5e1;
      border-radius: 12px;
      padding: 40px 20px;
      text-align: center;
      transition: all 0.3s ease;
      cursor: pointer;
      min-height: 200px;
      display: flex;
      flex-direction: column;
      justify-content: center;
      background: #f8fafc;
    }
    
    .drop-zone:hover {
      border-color: #10b981;
      background: #f0fdf4;
    }
    
    .drop-zone.drag-over {
      border-color: #10b981;
      background: #ecfdf5;
      transform: scale(1.02);
      border-style: solid;
    }
    
    .drop-icon {
      font-size: 48px;
      margin-bottom: 16px;
    }
    
    .drop-content h4 {
      margin: 0 0 8px 0;
      color: #1e293b;
      font-size: 16px;
      font-weight: 500;
    }
    
    .drop-hint {
      color: #64748b;
      margin: 0 0 20px 0;
      font-size: 14px;
    }
    
    .select-btn {
      background: #10b981;
      color: white;
      border: none;
      padding: 12px 24px;
      border-radius: 8px;
      cursor: pointer;
      font-weight: 500;
      font-size: 14px;
      transition: background 0.2s;
    }
    
    .select-btn:hover {
      background: #059669;
    }
    
    .file-list {
      margin-top: 24px;
      border-top: 1px solid #e2e8f0;
      padding-top: 20px;
    }
    
    .list-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 12px;
    }
    
    .list-header h4 {
      margin: 0;
      font-size: 14px;
      color: #475569;
      font-weight: 600;
    }
    
    .clear-all-btn {
      background: transparent;
      color: #64748b;
      border: none;
      padding: 4px 8px;
      cursor: pointer;
      font-size: 12px;
      text-decoration: underline;
    }
    
    .clear-all-btn:hover {
      color: #ef4444;
    }
    
    .files-container {
      max-height: 300px;
      overflow-y: auto;
      margin: 12px 0;
    }
    
    .file-item {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 12px;
      background: #f8fafc;
      border-radius: 8px;
      margin: 6px 0;
      transition: all 0.2s;
      border: 1px solid transparent;
    }
    
    .file-item:hover {
      background: #f1f5f9;
      border-color: #e2e8f0;
    }
    
    .file-icon {
      font-size: 20px;
      flex-shrink: 0;
    }
    
    .file-info {
      flex: 1;
      display: flex;
      flex-direction: column;
      gap: 4px;
      min-width: 0;
    }
    
    .file-name {
      font-size: 14px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      color: #1e293b;
      font-weight: 500;
    }
    
    .file-path {
      font-size: 11px;
      color: #94a3b8;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    
    .remove-btn {
      background: #fee2e2;
      color: #dc2626;
      border: none;
      width: 24px;
      height: 24px;
      border-radius: 50%;
      cursor: pointer;
      font-size: 12px;
      display: flex;
      align-items: center;
      justify-content: center;
      flex-shrink: 0;
      transition: all 0.2s;
    }
    
    .remove-btn:hover {
      background: #ef4444;
      color: white;
    }
    
    .action-buttons {
      margin-top: 16px;
    }
    
    .share-btn {
      background: linear-gradient(135deg, #10b981, #059669);
      color: white;
      border: none;
      padding: 12px 24px;
      border-radius: 8px;
      cursor: pointer;
      font-weight: 500;
      font-size: 14px;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
      transition: all 0.3s;
      width: 100%;
    }
    
    .share-btn:hover:not(:disabled) {
      transform: translateY(-1px);
      box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
    }
    
    .share-btn:disabled {
      background: #9ca3af;
      cursor: not-allowed;
    }
    
    .modal-overlay {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background: rgba(0, 0, 0, 0.5);
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 1000;
    }
    
    .modal-content {
      background: white;
      border-radius: 16px;
      padding: 32px;
      max-width: 500px;
      width: 90%;
      max-height: 80vh;
      overflow-y: auto;
    }
    
    .modal-content h3 {
      margin: 0 0 8px 0;
      font-size: 20px;
      color: #1e293b;
    }
    
    .modal-hint {
      margin: 0 0 24px 0;
      color: #64748b;
      font-size: 14px;
    }
    
    .device-selector-list {
      display: flex;
      flex-direction: column;
      gap: 12px;
      margin-bottom: 24px;
    }
    
    .device-selector-item {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 16px;
      background: #f8fafc;
      border: 2px solid #e2e8f0;
      border-radius: 12px;
      cursor: pointer;
      transition: all 0.2s;
      width: 100%;
      text-align: left;
    }
    
    .device-selector-item:hover {
      border-color: #10b981;
      background: #f0fdf4;
    }
    
    .device-details {
      flex: 1;
      display: flex;
      flex-direction: column;
      gap: 4px;
    }
    
    .device-details .device-name {
      font-weight: 600;
      color: #1e293b;
    }
    
    .device-ip {
      font-size: 12px;
      color: #64748b;
    }
    
    .arrow {
      font-size: 20px;
      color: #10b981;
    }
    
    .cancel-btn {
      background: #f1f5f9;
      color: #475569;
      border: none;
      padding: 12px 24px;
      border-radius: 8px;
      cursor: pointer;
      font-weight: 500;
      width: 100%;
    }
    
    .cancel-btn:hover {
      background: #e2e8f0;
    }
  </style>
  