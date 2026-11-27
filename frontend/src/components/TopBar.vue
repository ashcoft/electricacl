<script setup lang="ts">
import { computed, ref } from 'vue'
import { data, resetData, setScenario, toast, ui } from '../store'
import { SCENARIOS } from '../types'
import { api } from '../lib/api'
import Icon from './Icon.vue'
import { auth, signOut } from '../lib/auth'

const project = computed(() => data.state?.project)
const showScenarios = computed(() => ui.view === 'dashboard' || ui.view === 'loadlist')
const initials = computed(() => (auth.user ?? '?').slice(0, 1).toUpperCase())

const signingOut = ref(false)

async function logout() {
  if (signingOut.value) return
  signingOut.value = true
  const { revoked } = await signOut()
  resetData()
  signingOut.value = false
  if (!revoked) {
    toast(
      'error',
      'Signed out locally, but the server could not be reached - the session may still be active',
    )
  }
}
</script>

<template>
  <header class="topbar">
    <div class="topbar-title">
      <strong>{{ project?.document ?? 'Load list calculation' }}</strong>
      <span>{{ project?.name ?? 'Electrical' }} · {{ data.state?.stats.items ?? 0 }} items</span>
    </div>

    <div v-if="showScenarios" class="scenario-tabs">
      <button
        v-for="entry in SCENARIOS"
        :key="entry.key"
        class="scenario-tab"
        :class="{ active: ui.scenario === entry.key }"
        @click="setScenario(entry.key)"
      >
        {{ entry.short }}
      </button>
    </div>

    <div class="topbar-actions">
      <span class="user-chip" :title="`Signed in as ${auth.user ?? ''}`">
        <span class="avatar">{{ initials }}</span>
        <span>{{ auth.user }}</span>
      </span>
      <button
        class="btn icon"
        title="Calculation settings"
        aria-label="Calculation settings"
        @click="ui.settingsOpen = true"
      >
        <Icon name="settings" />
      </button>
      <a class="btn" :href="api.exportUrl" download title="Export the load list as CSV">
        <Icon name="download" :size="14" /> CSV
      </a>
      <a
        class="btn"
        :href="`${api.pdfUrl}?scenario=${ui.scenario}`"
        download
        title="Download the calculation report as PDF"
      >
        <Icon name="download" :size="14" /> PDF
      </a>
      <button class="btn primary" @click="ui.createOpen = true">
        <Icon name="plus" :size="14" /> New item
      </button>
      <button
        class="btn"
        title="Sign out"
        :disabled="signingOut"
        @click="logout"
      >
        <Icon name="power" :size="14" /> {{ signingOut ? 'Signing out…' : 'Sign out' }}
      </button>
    </div>
  </header>
</template>
