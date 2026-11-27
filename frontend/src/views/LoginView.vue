<script setup lang="ts">
import { nextTick, onMounted, ref } from 'vue'
import { auth, signIn } from '../lib/auth'

const username = ref('')
const password = ref('')
const capToken = ref<string | null>(null)
const widgetHost = ref<HTMLElement | null>(null)
const localError = ref('')

onMounted(async () => {
  if (!auth.capRequired) return
  try {
    // Registers the <cap-widget> custom element. Bundled, so the login page
    // keeps working on an air-gapped network.
    await import('cap-widget')
    await nextTick()
    const widget = widgetHost.value?.querySelector('cap-widget')
    widget?.addEventListener('solve', (event) => {
      const detail = (event as CustomEvent<{ token?: string }>).detail
      capToken.value = detail?.token ?? null
    })
    widget?.addEventListener('reset', () => {
      capToken.value = null
    })
  } catch (error) {
    localError.value =
      'The CAPTCHA widget could not be loaded. Reload the page or contact your administrator.'
    console.error(error)
  }
})

async function submit() {
  if (auth.busy) return
  localError.value = ''
  const ok = await signIn(username.value.trim(), password.value, capToken.value)
  if (ok) {
    password.value = ''
    return
  }
  // A used challenge cannot be replayed.
  const widget = widgetHost.value?.querySelector('cap-widget') as
    | (HTMLElement & { reset?: () => void })
    | null
  widget?.reset?.()
  capToken.value = null
}

function reset() {
  username.value = ''
  password.value = ''
  capToken.value = null
  localError.value = ''
  auth.error = ''
  const widget = widgetHost.value?.querySelector('cap-widget') as
    | (HTMLElement & { reset?: () => void })
    | null
  widget?.reset?.()
}
</script>

<template>
  <div class="login-wrapper">
    <section class="branding-panel">
      <div class="branding-content">
        <div class="eyebrow">Electrical Calculation Suite</div>
        <h1>Engineering numbers you can defend</h1>
        <p>
          Load list, cable sizing and cathodic protection in one workspace, with every value
          traceable back to its source and its formula shown next to it.
        </p>
        <div class="branding-points">
          <span>Load list</span>
          <span>Cable sizing</span>
          <span>ICCP / SACP</span>
          <span>Verified against the workbook</span>
        </div>
      </div>
    </section>

    <section class="login-panel">
      <div class="login-brand">
        <div class="brand-mark">⚡</div>
        <div>
          <div class="login-title">Electrical Calculation Suite</div>
          <div class="login-subtitle">Electrical Load List Calculation</div>
        </div>
      </div>

      <form @submit.prevent="submit">
        <div class="login-field">
          <label for="login-id">Login ID</label>
          <input
            id="login-id"
            v-model="username"
            class="login-input"
            placeholder="Enter your login ID"
            autocomplete="username"
            autofocus
          />
        </div>

        <div class="login-field">
          <label for="login-password">Password</label>
          <input
            id="login-password"
            v-model="password"
            type="password"
            class="login-input"
            placeholder="Enter your password"
            autocomplete="current-password"
          />
        </div>

        <div v-if="auth.capRequired" class="cap-box">
          <div ref="widgetHost" style="width: 100%">
            <cap-widget :data-cap-api-endpoint="auth.capEndpoint" />
          </div>
        </div>
        <div v-else class="cap-hint" style="margin: 0 0 14px">
          CAPTCHA is not configured for this deployment.
        </div>

        <div v-if="localError || auth.error" class="login-error">
          <span>⚠</span>
          <span>{{ localError || auth.error }}</span>
        </div>

        <button class="btn-login" type="submit" :disabled="auth.busy || !username || !password">
          {{ auth.busy ? 'Signing in…' : 'Sign In' }}
        </button>
        <button class="btn-reset" type="button" @click="reset">Reset</button>
      </form>

      <div class="footer-note">
        Session expires after 12 hours · Protected by
        <a href="https://github.com/tiagozip/cap" target="_blank" rel="noreferrer" style="color: var(--brand-dark)">Cap</a>
      </div>
    </section>
  </div>
</template>
