/**
 * Utility functions for hex encoding/decoding and formatting
 */

/**
 * Convert hex string to readable text
 * Preserves common whitespace characters (tab, newline, carriage return)
 * Other non-printable chars become dots
 */
export function hexToText(hex: string): string {
  let text = '';
  for (let i = 0; i < hex.length; i += 2) {
    const code = parseInt(hex.substr(i, 2), 16);
    // Printable ASCII characters
    if (code >= 32 && code <= 126) {
      text += String.fromCharCode(code);
    }
    // Tab, newline, carriage return - preserve these
    else if (code === 9) {
      text += '\t';
    }
    else if (code === 10) {
      text += '\n';
    }
    else if (code === 13) {
      text += '\r';
    }
    // Other non-printable characters become dots
    else {
      text += '.';
    }
  }
  return text;
}

/**
 * Convert text to hex string
 */
export function textToHex(text: string): string {
  return Array.from(text)
    .map(c => c.charCodeAt(0).toString(16).padStart(2, '0'))
    .join('');
}

/**
 * Format hex string with spaces for readability
 */
export function formatHex(hex: string): string {
  return hex.toUpperCase().match(/.{1,2}/g)?.join(' ') || '';
}

/**
 * Validate hex string
 */
export function isValidHex(str: string): boolean {
  return /^[0-9a-fA-F]*$/.test(str) && str.length % 2 === 0;
}

/**
 * Format bytes to human readable string
 */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / (1024 * 1024)).toFixed(2) + ' MB';
}

/**
 * Format timestamp to time string
 */
export function formatTime(date: Date): string {
  const hours = date.getHours().toString().padStart(2, '0');
  const minutes = date.getMinutes().toString().padStart(2, '0');
  const seconds = date.getSeconds().toString().padStart(2, '0');
  const ms = date.getMilliseconds().toString().padStart(3, '0');
  return `${hours}:${minutes}:${seconds}.${ms}`;
}

/**
 * Generate unique ID
 */
export function generateId(): string {
  return Date.now().toString(36) + Math.random().toString(36).substr(2);
}

/**
 * Copy text to clipboard
 */
export async function copyToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}
