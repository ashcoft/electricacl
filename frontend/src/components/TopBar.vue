<script setup lang="ts">
import { computed } from 'vue'
import { data, resetData, setScenario, ui } from '../store'
import { SCENARIOS } from '../types'
import { api } from '../lib/api'
import { auth, signOut } from '../lib/auth'

const project = computed(() => data.state?.project)
const showScenarios = computed(() => ui.view === 'dashboard' || ui.view === 'loadlist')
const initials = computed(() => (auth.user ?? '?').slice(0, 1).toUpperCase())

async function logout() {
  await signOut()
  resetData()
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
      <button class="btn icon" title="Calculation settings" @click="ui.settingsOpen = true">
        ⚙
      </button>
      <a class="btn" :href="api.exportUrl" download title="Export the load list as CSV">
        ↓ CSV
      </a>
      <button class="btn primary" @click="ui.createOpen = true">+ New item</button>
      <button class="btn icon" title="Sign out" @click="logout">⏻</button>
    </div>
  </header>
</template>
