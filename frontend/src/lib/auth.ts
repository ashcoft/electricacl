import { reactive } from 'vue'
import { api, onUnauthorized } from './api'
import type { AuthStatus } from '../types'

export const auth = reactive({
  ready: false,
  authenticated: false,
  user: null as string | null,
  capEndpoint: null as string | null,
  capRequired: false,
  error: '',
  busy: false,
})

function apply(status: AuthStatus) {
  auth.authenticated = status.authenticated
  auth.user = status.user
  auth.capEndpoint = status.cap_endpoint
  auth.capRequired = status.cap_required
}

export async function refreshAuth() {
  try {
    apply(await api.authStatus())
  } catch (error) {
    auth.error = error instanceof Error ? error.message : String(error)
  } finally {
    auth.ready = true
  }
}

export async function signIn(username: string, password: string, capToken: string | null) {
  auth.busy = true
  auth.error = ''
  try {
    const result = await api.login(username, password, capToken)
    auth.authenticated = true
    auth.user = result.user
    return true
  } catch (error) {
    auth.error = error instanceof Error ? error.message : String(error)
    return false
  } finally {
    auth.busy = false
  }
}

export async function signOut(): Promise<{ revoked: boolean }> {
  let revoked = true
  try {
    await api.logout()
  } catch (error) {
    // The UI signs out locally either way, but the token may still be valid on
    // the server, so say so instead of failing silently.
    revoked = false
    console.warn('sign-out request failed', error)
  }
  auth.authenticated = false
  auth.user = null
  auth.error = ''
  return { revoked }
}

onUnauthorized(() => {
  if (auth.authenticated) {
    auth.authenticated = false
    auth.user = null
    auth.error = 'Your session expired, please sign in again'
  }
})
