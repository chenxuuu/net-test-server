import { ref, reactive, readonly } from 'vue';
import type { Client, MessageEntry, WsMessage } from '../types';
import { generateId, textToHex } from '../utils';

export function useWebSocket() {
  const ws = ref<WebSocket | null>(null);
  const connected = ref(false);
  const port = ref<number | null>(null);
  const clients = reactive<Map<string, Client>>(new Map());
  const selectedClient = ref<string | null>(null);
  const messages = ref<MessageEntry[]>([]);
  
  const stats = reactive({
    messageCount: 0,
    rxBytes: 0,
    txBytes: 0,
  });

  let reconnectAttempts = 0;
  const maxReconnectAttempts = 5;
  const reconnectDelay = 2000;
  let heartbeatTimer: ReturnType<typeof setInterval> | null = null;

  function connect() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws/netlab`;
    
    ws.value = new WebSocket(wsUrl);
    
    ws.value.onopen = () => {
      reconnectAttempts = 0;
      connected.value = true;
      addSystemMessage('已连接到服务器');
      heartbeatTimer = setInterval(() => {
        if (ws.value && ws.value.readyState === WebSocket.OPEN) {
          ws.value.send('{}');
        }
      }, 30000);
    };
    
    ws.value.onclose = () => {
      connected.value = false;
      if (heartbeatTimer !== null) {
        clearInterval(heartbeatTimer);
        heartbeatTimer = null;
      }
      handleDisconnect();
    };
    
    ws.value.onerror = (error) => {
      console.error('WebSocket error:', error);
      addErrorMessage('连接发生错误');
    };
    
    ws.value.onmessage = (event) => {
      handleMessage(event.data);
    };
  }

  function handleDisconnect() {
    port.value = null;
    clients.clear();
    
    if (reconnectAttempts < maxReconnectAttempts) {
      reconnectAttempts++;
      addSystemMessage(`连接断开，${reconnectDelay / 1000}秒后尝试重连 (${reconnectAttempts}/${maxReconnectAttempts})`);
      setTimeout(connect, reconnectDelay);
    } else {
      addErrorMessage('无法连接到服务器，请刷新页面重试');
    }
  }

  function handleMessage(data: string) {
    try {
      const msg = JSON.parse(data) as WsMessage;
      
      switch (msg.action) {
        case 'port':
          port.value = msg.port;
          addSystemMessage(`TCP端口 ${msg.port} 已创建，等待设备连接...`);
          break;
          
        case 'connected':
          clients.set(msg.client, {
            id: msg.client,
            addr: msg.addr,
            connectedAt: new Date(),
          });
          addSystemMessage(`设备 [${msg.client}] 已连接 (${msg.addr})`);
          break;
          
        case 'closed':
          clients.delete(msg.client);
          if (selectedClient.value === msg.client) {
            selectedClient.value = null;
          }
          addSystemMessage(`设备 [${msg.client}] 已断开连接`);
          break;
          
        case 'data':
          const byteCount = msg.hex ? msg.data.length / 2 : msg.data.length;
          stats.rxBytes += byteCount;
          addDataMessage(msg.client, msg.data, 'incoming', msg.hex);
          break;
          
        case 'error':
          addErrorMessage(msg.msg);
          break;
      }
    } catch (e) {
      console.error('Failed to parse message:', e);
    }
  }

  function createPort() {
    if (!ws.value || ws.value.readyState !== WebSocket.OPEN) {
      addErrorMessage('未连接到服务器');
      return;
    }
    
    ws.value.send(JSON.stringify({
      action: 'newp',
      type: 'tcp',
    }));
  }

  function sendData(clientId: string, data: string, isHex: boolean) {
    if (!ws.value || ws.value.readyState !== WebSocket.OPEN) {
      addErrorMessage('未连接到服务器');
      return false;
    }
    
    const hexData = isHex ? data.toLowerCase() : textToHex(data);
    
    ws.value.send(JSON.stringify({
      action: 'sendc',
      client: clientId,
      data: hexData,
      hex: true,
    }));
    
    const byteCount = hexData.length / 2;
    stats.txBytes += byteCount;
    addDataMessage(clientId, hexData, 'outgoing', true);
    
    return true;
  }

  function disconnectClient(clientId: string) {
    if (!ws.value || ws.value.readyState !== WebSocket.OPEN) return;
    
    ws.value.send(JSON.stringify({
      action: 'closec',
      client: clientId,
    }));
  }

  function addSystemMessage(text: string) {
    stats.messageCount++;
    messages.value.push({
      id: generateId(),
      type: 'system',
      data: text,
      timestamp: new Date(),
    });
  }

  function addDataMessage(clientId: string, data: string, type: 'incoming' | 'outgoing', hex: boolean) {
    stats.messageCount++;
    messages.value.push({
      id: generateId(),
      type,
      clientId,
      data,
      hex,
      timestamp: new Date(),
    });
  }

  function addErrorMessage(text: string) {
    stats.messageCount++;
    messages.value.push({
      id: generateId(),
      type: 'error',
      data: text,
      timestamp: new Date(),
    });
  }

  function clearMessages() {
    messages.value = [];
  }

  function selectClient(clientId: string | null) {
    selectedClient.value = clientId;
  }

  return {
    // State
    connected: readonly(connected),
    port: readonly(port),
    clients: readonly(clients),
    selectedClient: readonly(selectedClient),
    messages: readonly(messages),
    stats: readonly(stats),
    
    // Actions
    connect,
    createPort,
    sendData,
    disconnectClient,
    clearMessages,
    selectClient,
  };
}
