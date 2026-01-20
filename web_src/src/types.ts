// WebSocket Message Types
export interface WsPortCreated {
  action: 'port';
  port: number;
}

export interface WsClientConnected {
  action: 'connected';
  client: string;
  addr: string;
}

export interface WsClientDisconnected {
  action: 'closed';
  client: string;
}

export interface WsDataReceived {
  action: 'data';
  client: string;
  data: string;
  hex: boolean;
}

export interface WsError {
  action: 'error';
  msg: string;
}

export type WsMessage = 
  | WsPortCreated 
  | WsClientConnected 
  | WsClientDisconnected 
  | WsDataReceived 
  | WsError;

// Client state
export interface Client {
  id: string;
  addr: string;
  connectedAt: Date;
}

// Message log entry
export interface MessageEntry {
  id: string;
  type: 'system' | 'incoming' | 'outgoing' | 'error';
  clientId?: string;
  data: string;
  hex?: boolean;
  timestamp: Date;
}

// App state
export interface AppState {
  connected: boolean;
  port: number | null;
  clients: Map<string, Client>;
  selectedClient: string | null;
  messages: MessageEntry[];
  stats: {
    messageCount: number;
    rxBytes: number;
    txBytes: number;
  };
}
