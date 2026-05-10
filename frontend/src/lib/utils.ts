export function unwrap<T>(payload: unknown): T {
  if (payload && typeof payload === 'object' && 'data' in payload) {
    return (payload as { data: T }).data;
  }
  return payload as T;
}

export async function fetchJson<T = unknown>(url: string, init?: RequestInit): Promise<T> {
  const response = await fetch(url, init);
  const text = await response.text();
  const payload = text ? JSON.parse(text) : null;

  if (!response.ok) {
    const message = payload && typeof payload === 'object'
      ? (payload.message || payload.error)
      : null;
    throw new Error(message || `Request failed (${response.status})`);
  }

  return payload as T;
}

export function formatMonth(period: string) {
  const date = new Date(`${period}T00:00:00`);
  return Number.isNaN(date.getTime())
    ? period.slice(0, 7)
    : date.toLocaleString(undefined, { month: 'short', year: '2-digit' });
}

export function formatHour(timestamp: string) {
  const date = new Date(timestamp);
  return Number.isNaN(date.getTime())
    ? timestamp.slice(11, 16)
    : date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
}

export function formatDateTime(timestamp: string) {
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) return timestamp;
  return date.toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

