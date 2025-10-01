<script lang="ts">
    export let transfers: any[] = [];
    
    function getStatusColor(status: string): string {
      if (status.includes('Completed')) return '#10b981';
      if (status.includes('Sending') || status.includes('Receiving')) return '#3b82f6';
      if (status.includes('Failed')) return '#ef4444';
      return '#6b7280';
    }
    
    function formatBytes(bytes: number): string {
      if (bytes === 0) return '0 Bytes';
      const k = 1024;
      const sizes = ['Bytes', 'KB', 'MB', 'GB'];
      const i = Math.floor(Math.log(bytes) / Math.log(k));
      return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i];
    }
    
    function getProgressPercentage(transfer: any): number {
      if (transfer.size === 0) return 0;
      return Math.round((transfer.progress / transfer.size) * 100);
    }
  </script>
  
  <div class="panel">
    <h3>📜 Transfer History</h3>
    
    {#if transfers.length === 0}
      <div class="empty-state">
        <div class="empty-icon">📭</div>
        <p>No transfers yet</p>
        <span class="hint">Encrypted transfers will appear here</span>
      </div>
    {:else}
      <div class="transfer-list">
        {#each transfers as transfer (transfer.id)}
          <div class="transfer-item">
            <div class="transfer-header">
              <div class="filename-row">
                <span class="transfer-filename">{transfer.filename}</span>
                {#if transfer.encrypted}
                  <span class="encrypted-icon" title="Encrypted">🔐</span>
                {/if}
              </div>
              <span class="transfer-status" style="color: {getStatusColor(transfer.status)}">
                {transfer.status}
              </span>
            </div>
            
            <div class="transfer-details">
              <span class="transfer-size">{formatBytes(transfer.size)}</span>
              <span class="transfer-direction">
                {transfer.status.includes('Sending') ? '→' : '←'}
                {transfer.status.includes('Sending') ? transfer.to_device : transfer.from_device}
              </span>
            </div>
            
            {#if transfer.status.includes('Sending') || transfer.status.includes('Receiving')}
              <div class="progress-bar">
                <div class="progress-fill" style="width: {getProgressPercentage(transfer)}%"></div>
              </div>
              <span class="progress-text">{getProgressPercentage(transfer)}%</span>
            {/if}
          </div>
        {/each}
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
      max-height: calc(100vh - 140px);
      overflow-y: auto;
    }
    
    .panel h3 {
      margin: 0 0 20px 0;
      font-size: 18px;
      color: #1e293b;
      font-weight: 600;
      position: sticky;
      top: 0;
      background: white;
      padding-bottom: 12px;
      z-index: 1;
    }
    
    .empty-state {
      text-align: center;
      padding: 60px 20px;
      color: #64748b;
    }
    
    .empty-icon {
      font-size: 48px;
      margin-bottom: 16px;
    }
    
    .empty-state p {
      margin: 0 0 8px 0;
      font-size: 16px;
      color: #1e293b;
      font-weight: 500;
    }
    
    .hint {
      font-size: 12px;
      color: #94a3b8;
    }
    
    .transfer-list {
      display: flex;
      flex-direction: column;
      gap: 12px;
    }
    
    .transfer-item {
      padding: 16px;
      background: #f8fafc;
      border: 1px solid #e2e8f0;
      border-radius: 8px;
      transition: all 0.2s;
    }
    
    .transfer-item:hover {
      border-color: #cbd5e1;
      box-shadow: 0 1px 2px rgba(0,0,0,0.05);
    }
    
    .transfer-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 8px;
    }
    
    .filename-row {
      display: flex;
      align-items: center;
      gap: 8px;
      flex: 1;
      margin-right: 12px;
    }
    
    .transfer-filename {
      font-weight: 500;
      font-size: 14px;
      color: #1e293b;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    
    .encrypted-icon {
      font-size: 12px;
      flex-shrink: 0;
    }
    
    .transfer-status {
      font-size: 11px;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      white-space: nowrap;
    }
    
    .transfer-details {
      display: flex;
      justify-content: space-between;
      align-items: center;
      font-size: 12px;
      color: #64748b;
      margin-bottom: 8px;
    }
    
    .transfer-size {
      font-weight: 500;
    }
    
    .transfer-direction {
      display: flex;
      align-items: center;
      gap: 4px;
    }
    
    .progress-bar {
      height: 4px;
      background: #e2e8f0;
      border-radius: 2px;
      overflow: hidden;
      margin-bottom: 4px;
    }
    
    .progress-fill {
      height: 100%;
      background: linear-gradient(90deg, #10b981 0%, #059669 100%);
      transition: width 0.3s ease;
    }
    
    .progress-text {
      font-size: 11px;
      color: #10b981;
      font-weight: 600;
    }
  </style>
  