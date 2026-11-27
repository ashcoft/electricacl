import type { AppState, AuthStatus, CalcSettings, LoadItem } from '../types'

/** Called whenever the API rejects a request because the session is gone. */
let unauthorizedHandler: (() => void) | null = null

export function onUnauthorized(handler: () => void) {
  unauthorizedHandler = handler
}

async function request<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await fetch(url, {
    headers: { 'Content-Type': 'application/json' },
    ...init,
  })
  if (response.status === 401) {
    unauthorizedHandler?.()
  }
  if (!response.ok) {
    let message = `${response.status} ${response.statusText}`
    try {
      const body = (await response.json()) as { error?: string }
      if (body.error) message = body.error
    } catch {
      /* response without JSON body */
    }
    throw new Error(message)
  }
  if (response.status === 204) return undefined as T
  return (await response.json()) as T
}

export const api = {
  authStatus: () => request<AuthStatus>('/api/auth/config'),

  login: (username: string, password: string, capToken: string | null) =>
    request<{ user: string }>('/api/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username, password, cap_token: capToken }),
    }),

  logout: () => request<{ ok: boolean }>('/api/auth/logout', { method: 'POST' }),

  getState: () => request<AppState>('/api/state'),

  saveSettings: (settings: CalcSettings) =>
    request<CalcSettings>('/api/settings', {
      method: 'PUT',
      body: JSON.stringify(settings),
    }),

  createItem: (item: Partial<LoadItem>) =>
    request<LoadItem>('/api/load-items', {
      method: 'POST',
      body: JSON.stringify(item),
    }),

  updateItem: (item: LoadItem) =>
    request<LoadItem>(`/api/load-items/${item.id}`, {
      method: 'PUT',
      body: JSON.stringify(item),
    }),

  deleteItem: (id: string) =>
    request<void>(`/api/load-items/${id}`, { method: 'DELETE' }),

  duplicateItem: (id: string) =>
    request<LoadItem>(`/api/load-items/${id}/duplicate`, { method: 'POST' }),

  resetItems: () =>
    request<{ restored: number }>('/api/load-items/reset', { method: 'POST' }),

  exportUrl: '/api/export/csv',
  pdfUrl: '/api/export/pdf',
}
