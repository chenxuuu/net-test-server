<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue';
import { useWebSocket } from './composables/useWebSocket';
import { formatHex, hexToText, isValidHex, formatBytes, formatTime, copyToClipboard } from './utils';

// WebSocket连接
const {
  connected,
  port,
  clients,
  selectedClient,
  messages,
  stats,
  connect,
  createPort,
  sendData,
  disconnectClient,
  clearMessages,
  selectClient,
} = useWebSocket();

// IP地址配置
const ipv4 = ref('152.70.80.204');
const ipv6 = ref('2603:c023:1:5fcc:c028:8ed:49a7:6e08');

// 发送表单
const sendInput = ref('');
const dataFormat = ref<'hex' | 'text'>('hex');
const messageContainerRef = ref<HTMLElement | null>(null);

// 显示模式: 'hex' 或 'ascii'
const displayMode = ref<'hex' | 'ascii'>('hex');

// 当前时间
const currentTime = ref(new Date().toLocaleTimeString('zh-CN'));

// 计算属性
const clientArray = computed(() => Array.from(clients.values()));

// 处理发送
function handleSend() {
  if (!selectedClient.value) {
    showToast('warning', '请先选择目标设备');
    return;
  }
  
  const data = sendInput.value.trim();
  if (!data) {
    showToast('warning', '请输入要发送的数据');
    return;
  }
  
  const isHex = dataFormat.value === 'hex';
  
  if (isHex) {
    const cleanHex = data.replace(/\s/g, '');
    if (!isValidHex(cleanHex)) {
      showToast('error', 'HEX格式错误');
      return;
    }
    if (sendData(selectedClient.value, cleanHex, true)) {
      sendInput.value = '';
    }
  } else {
    if (sendData(selectedClient.value, data, false)) {
      sendInput.value = '';
    }
  }
}

// 快捷发送
function quickSend(hex: string) {
  sendInput.value = hex;
  dataFormat.value = 'hex';
}

// 复制到剪贴板
async function copyText(text: string) {
  if (await copyToClipboard(text)) {
    showToast('success', '已复制到剪贴板');
  }
}

// Toast 通知
interface Toast {
  id: number;
  type: 'success' | 'error' | 'warning' | 'info';
  message: string;
  hiding?: boolean;
}
const toasts = ref<Toast[]>([]);
let toastId = 0;

function showToast(type: Toast['type'], message: string) {
  const id = ++toastId;
  toasts.value.push({ id, type, message });
  
  setTimeout(() => {
    const toast = toasts.value.find(t => t.id === id);
    if (toast) toast.hiding = true;
    setTimeout(() => {
      toasts.value = toasts.value.filter(t => t.id !== id);
    }, 200);
  }, 3000);
}

// 自动滚动到底部
watch(messages, () => {
  nextTick(() => {
    if (messageContainerRef.value) {
      messageContainerRef.value.scrollTop = messageContainerRef.value.scrollHeight;
    }
  });
}, { deep: true });

// 时钟更新
let clockInterval: number;
onMounted(() => {
  connect();
  clockInterval = window.setInterval(() => {
    currentTime.value = new Date().toLocaleTimeString('zh-CN');
  }, 1000);
});

onUnmounted(() => {
  clearInterval(clockInterval);
});
</script>

<template>
  <div class="app">
    <!-- Header -->
    <header class="header">
      <div class="header-content">
        <div class="logo">
          <svg class="logo-icon" viewBox="0 0 24 24" fill="none">
            <rect x="2" y="3" width="20" height="14" rx="2" stroke="currentColor" stroke-width="2"/>
            <path d="M8 21H16" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            <path d="M12 17V21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            <circle cx="12" cy="10" r="3" stroke="currentColor" stroke-width="2"/>
          </svg>
          <span class="logo-text">在线TCP调试工具 支持IPV6</span>
        </div>
        
        <div class="status-indicator" :class="{ connected }">
          <div class="status-led"></div>
          <span class="status-text">{{ connected ? '在线' : '离线' }}</span>
        </div>
      </div>
    </header>

    <!-- Main -->
    <main class="main">
      <!-- Left Sidebar - Connection -->
      <aside class="sidebar">
        <div class="panel">
          <div class="panel-header">
              <h2 class="panel-title">
              <svg class="icon" viewBox="0 0 24 24" fill="none">
                <path d="M22 12H18L15 21L9 3L6 12H2" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
                控制面板
            </h2>
          </div>
          
          <div class="panel-content">
            <!-- Port Module -->
            <div v-if="port" class="port-module">
              <div class="port-module-header">
                <span class="port-module-title">TCP 端口</span>
                <div class="port-module-status">
                  <div class="port-module-led"></div>
                </div>
              </div>
              <div class="port-module-body">
                <div class="port-display">
                  <div class="port-number copyable" @click="copyText(String(port))" title="点击复制端口号">{{ port }}</div>
                </div>
                <div class="port-addresses">
                  <div class="port-address" @click="copyText(ipv4)" title="点击复制">
                    <span class="addr-label">IPv4</span>
                    <span class="addr-value">{{ ipv4 }}</span>
                    <svg class="copy-icon" viewBox="0 0 24 24" fill="none">
                      <rect x="9" y="9" width="13" height="13" rx="2" stroke="currentColor" stroke-width="2"/>
                      <path d="M5 15H4C2.89543 15 2 14.1046 2 13V4C2 2.89543 2.89543 2 4 2H13C14.1046 2 15 2.89543 15 4V5" stroke="currentColor" stroke-width="2"/>
                    </svg>
                  </div>
                  <div class="port-address" @click="copyText(ipv6)" title="点击复制">
                    <span class="addr-label">IPv6</span>
                    <span class="addr-value">{{ ipv6 }}</span>
                    <svg class="copy-icon" viewBox="0 0 24 24" fill="none">
                      <rect x="9" y="9" width="13" height="13" rx="2" stroke="currentColor" stroke-width="2"/>
                      <path d="M5 15H4C2.89543 15 2 14.1046 2 13V4C2 2.89543 2.89543 2 4 2H13C14.1046 2 15 2.89543 15 4V5" stroke="currentColor" stroke-width="2"/>
                    </svg>
                  </div>
                </div>
              </div>
            </div>
            
            <!-- Create Port Button -->
            <button 
              v-else 
              class="btn btn-primary btn-block" 
              @click="createPort"
              :disabled="!connected"
            >
              <svg class="icon" viewBox="0 0 24 24" fill="none">
                <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2"/>
                <path d="M12 8V16M8 12H16" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              </svg>
              创建 TCP 端口
            </button>

            <!-- Client List -->
            <div class="client-section">
              <div class="section-header">
                <span class="section-title">已连接设备</span>
                <span class="client-count">{{ clientArray.length }}</span>
              </div>
              
              <div class="client-list">
                <template v-if="clientArray.length > 0">
                  <div 
                    v-for="client in clientArray" 
                    :key="client.id"
                    class="client-item"
                    :class="{ selected: selectedClient === client.id }"
                    @click="selectClient(client.id)"
                  >
                    <div class="client-info">
                      <span class="client-id">{{ client.id }}</span>
                      <span class="client-addr">{{ client.addr }}</span>
                    </div>
                    <button 
                      class="btn btn-danger btn-sm" 
                      @click.stop="disconnectClient(client.id)"
                      title="断开连接"
                    >
                      <svg class="icon-sm" viewBox="0 0 24 24" fill="none">
                        <path d="M18 6L6 18M6 6L18 18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                      </svg>
                    </button>
                  </div>
                </template>
                
                <div v-else class="empty-state">
                  <svg class="empty-icon" viewBox="0 0 24 24" fill="none">
                    <path d="M17 21V19C17 16.79 15.21 15 13 15H5C2.79 15 1 16.79 1 19V21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                    <circle cx="9" cy="7" r="4" stroke="currentColor" stroke-width="2"/>
                    <path d="M23 21V19C23 17.14 21.73 15.57 20 15.13" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                    <path d="M16 3.13C17.73 3.57 19 5.14 19 7C19 8.86 17.73 10.43 16 10.87" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                  </svg>
                  <p class="empty-text">暂无设备连接</p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </aside>

      <!-- Content - Message Log -->
      <section class="content">
        <div class="panel message-panel">
          <div class="panel-header">
            <h2 class="panel-title">
              <svg class="icon" viewBox="0 0 24 24" fill="none">
                <path d="M21 15C21 15.53 20.79 16.04 20.41 16.41C20.04 16.79 19.53 17 19 17H7L3 21V5C3 4.47 3.21 3.96 3.59 3.59C3.96 3.21 4.47 3 5 3H19C19.53 3 20.04 3.21 20.41 3.59C20.79 3.96 21 4.47 21 5V15Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              日志信息
            </h2>
            <div style="display: flex; gap: 8px; align-items: center;">
              <!-- 显示模式切换 -->
              <div class="display-mode-toggle">
                <button 
                  class="display-mode-btn" 
                  :class="{ active: displayMode === 'hex' }"
                  @click="displayMode = 'hex'"
                >HEX</button>
                <button 
                  class="display-mode-btn" 
                  :class="{ active: displayMode === 'ascii' }"
                  @click="displayMode = 'ascii'"
                >ASCII</button>
              </div>
              <button class="btn btn-ghost btn-sm" @click="clearMessages" title="清空日志">
                <svg class="icon-sm" viewBox="0 0 24 24" fill="none">
                  <path d="M3 6H21M19 6V20C19 21.1 18.1 22 17 22H7C5.9 22 5 21.1 5 20V6M8 6V4C8 2.9 8.9 2 10 2H14C15.1 2 16 2.9 16 4V6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </button>
            </div>
          </div>
          
          <div class="message-container" ref="messageContainerRef">
            <template v-if="messages.length > 0">
              <div 
                v-for="msg in messages" 
                :key="msg.id" 
                class="message-item"
                :class="msg.type"
              >
                <div class="message-header">
                  <div class="message-tag">
                    <span class="message-type">
                      {{ msg.type === 'incoming' ? '接收' : msg.type === 'outgoing' ? '发送' : msg.type === 'system' ? '系统' : '错误' }}
                    </span>
                    <span v-if="msg.clientId" class="message-client">[{{ msg.clientId }}]</span>
                  </div>
                  <span class="message-time">{{ formatTime(msg.timestamp) }}</span>
                </div>
                <!-- HEX/ASCII 根据模式显示 -->
                <template v-if="msg.hex">
                  <div v-if="displayMode === 'hex'" class="message-content hex">
                    {{ formatHex(msg.data) }}
                  </div>
                  <div v-else class="message-content">
                    {{ hexToText(msg.data) }}
                  </div>
                  <!-- 折叠的另一种显示 -->
                  <div 
                    class="message-text-preview" 
                    :class="{ collapsed: true }" 
                    style="display: none;"
                  >
                    {{ displayMode === 'hex' ? 'ASCII 码: ' + hexToText(msg.data) : '十六进制: ' + formatHex(msg.data) }}
                  </div>
                </template>
                <template v-else>
                  <div class="message-content">{{ msg.data }}</div>
                </template>
              </div>
            </template>
            
            <div v-else class="welcome-screen">
              <svg class="welcome-icon" viewBox="0 0 24 24" fill="none">
                <rect x="2" y="3" width="20" height="14" rx="2" stroke="currentColor" stroke-width="2"/>
                <path d="M8 21H16" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                <path d="M12 17V21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                <path d="M7 8L10 11L7 14" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                <path d="M12 14H17" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              </svg>
              <h3 class="welcome-title">TCP 调试控制台</h3>
              <p class="welcome-subtitle">点击 "创建 TCP 端口" 开始调试</p>
            </div>
          </div>
        </div>
      </section>

      <!-- Right Sidebar - Send -->
      <aside class="sidebar sidebar-right">
        <div class="panel">
          <div class="panel-header">
            <h2 class="panel-title">
              <svg class="icon" viewBox="0 0 24 24" fill="none">
                <path d="M22 2L11 13" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                <path d="M22 2L15 22L11 13L2 9L22 2Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              发送
            </h2>
          </div>
          
          <div class="panel-content">
            <div class="send-form">
              <div class="form-group">
                <label class="form-label">目标设备</label>
                <select 
                  class="form-select" 
                  :value="selectedClient || ''"
                  @change="selectClient(($event.target as HTMLSelectElement).value || null)"
                >
                  <option value="">-- 请选择 --</option>
                  <option 
                    v-for="client in clientArray" 
                    :key="client.id" 
                    :value="client.id"
                  >
                    {{ client.id }} ({{ client.addr }})
                  </option>
                </select>
              </div>
              
              <div class="form-group">
                <label class="form-label">数据格式</label>
                <div class="radio-group">
                  <label class="radio-item">
                    <input type="radio" v-model="dataFormat" value="hex">
                    <span class="radio-label">十六进制</span>
                  </label>
                  <label class="radio-item">
                    <input type="radio" v-model="dataFormat" value="text">
                    <span class="radio-label">文本</span>
                  </label>
                </div>
              </div>
              
              <div class="form-group">
                <label class="form-label">数据</label>
                <textarea 
                  class="form-textarea" 
                  v-model="sendInput"
                  :placeholder="dataFormat === 'hex' ? 'HEX: 48 65 6C 6C 6F' : 'TEXT: Hello'"
                  @keydown.ctrl.enter="handleSend"
                ></textarea>
              </div>
              
              <button class="btn btn-success btn-block" @click="handleSend" :disabled="!selectedClient">
                <svg class="icon" viewBox="0 0 24 24" fill="none">
                  <path d="M22 2L11 13" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                  <path d="M22 2L15 22L11 13L2 9L22 2Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                发送
              </button>
            </div>
            
            <div class="quick-send">
              <div class="section-header">
                <span class="section-title">快速发送</span>
              </div>
              <div class="quick-buttons">
                <button class="btn btn-sm" @click="quickSend('48656c6c6f')">Hello</button>
                <button class="btn btn-sm" @click="quickSend('0d0a')">CRLF</button>
                <button class="btn btn-sm" @click="quickSend('00')">NULL</button>
                <button class="btn btn-sm" @click="quickSend('ff')">0xFF</button>
                <button class="btn btn-sm" @click="quickSend('414243')">ABC</button>
              </div>
            </div>
          </div>
        </div>
      </aside>
    </main>

    <!-- Footer -->
    <footer class="footer">
      <div class="footer-content">
        <div class="footer-item">
          <svg class="icon" viewBox="0 0 24 24" fill="none">
            <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2"/>
            <path d="M12 6V12L16 14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
          {{ currentTime }}
        </div>
        <div class="footer-item">
          by 晨旭 | 反馈问题: http://github.com/chenxuuu/net-test-server
        </div>
        
        <div class="footer-stats">
          <span class="footer-item">
            <span class="stat-label">消息:</span>
            <span class="stat-value">{{ stats.messageCount }}</span>
          </span>
          <span class="footer-item">
            <span class="stat-label">接收:</span>
            <span class="stat-value">{{ formatBytes(stats.rxBytes) }}</span>
          </span>
          <span class="footer-item">
            <span class="stat-label">发送:</span>
            <span class="stat-value">{{ formatBytes(stats.txBytes) }}</span>
          </span>
        </div>
      </div>
    </footer>

    <!-- Toast Container -->
    <div class="toast-container">
      <div 
        v-for="toast in toasts" 
        :key="toast.id" 
        class="toast"
        :class="[toast.type, { hiding: toast.hiding }]"
      >
        <svg class="toast-icon" viewBox="0 0 24 24" fill="none">
          <template v-if="toast.type === 'success'">
            <path d="M22 11.08V12C22 17.52 17.52 22 12 22C6.48 22 2 17.52 2 12C2 6.48 6.48 2 12 2C13.28 2 14.5 2.26 15.64 2.74" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            <path d="M22 4L12 14.01L9 11.01" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </template>
          <template v-else-if="toast.type === 'error'">
            <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2"/>
            <path d="M15 9L9 15M9 9L15 15" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </template>
          <template v-else-if="toast.type === 'warning'">
            <path d="M10.29 3.86L1.82 18C1.64 18.3 1.55 18.65 1.55 19C1.55 20.1 2.45 21 3.55 21H20.45C21.55 21 22.45 20.1 22.45 19C22.45 18.65 22.36 18.3 22.18 18L13.71 3.86C13.15 2.85 11.85 2.85 10.29 3.86Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M12 9V13M12 17H12.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </template>
          <template v-else>
            <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2"/>
            <path d="M12 16V12M12 8H12.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </template>
        </svg>
        <span class="toast-message">{{ toast.message }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Component-specific styles if needed */
</style>
