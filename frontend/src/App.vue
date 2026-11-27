<script setup lang="ts">
import { onBeforeUnmount, onMounted, watch } from 'vue'
import CableSizingView from './views/CableSizingView.vue'
import CableDrawer from './components/CableDrawer.vue'
import CathodicView from './views/CathodicView.vue'
import CathodicDrawer from './components/CathodicDrawer.vue'
import DashboardView from './views/DashboardView.vue'
import LoadListView from './views/LoadListView.vue'
import LoginView from './views/LoginView.vue'
import MethodView from './views/MethodView.vue'
import MotorTableView from './views/MotorTableView.vue'
import ItemDrawer from './components/ItemDrawer.vue'
import NavRail from './components/NavRail.vue'
import NewItemDialog from './components/NewItemDialog.vue'
import SettingsDialog from './components/SettingsDialog.vue'
import Toasts from './components/Toasts.vue'
import TopBar from './components/TopBar.vue'
import { auth, refreshAuth } from './lib/auth'
import { load, resetData, ui } from './store'

function onKeydown(event: KeyboardEvent) {
  const target = event.target as HTMLElement | null
  const typing =
    target &&
    (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT')

  if (event.key === 'Escape') {
    if (ui.createOpen) ui.createOpen = false
    else if (ui.cableCreateOpen) ui.cableCreateOpen = false
    else if (ui.settingsOpen) ui.settingsOpen = false
    else if (ui.drawerId) ui.drawerId = null
    else if (ui.cableDrawerId) ui.cableDrawerId = null
    else if (ui.cathodicDrawerId) ui.cathodicDrawerId = null
    return
  }
  if (typing || !auth.authenticated) return
  if (event.key === '/') {
    event.preventDefault()
    ui.view = 'loadlist'
    window.setTimeout(() => {
      const input = document.querySelector<HTMLInputElement>('[data-search]')
      input?.focus()
      input?.select()
    }, 30)
  }
  if (event.key === 'n' || event.key === 'N') {
    ui.createOpen = true
  }
}

onMounted(async () => {
  window.addEventListener('keydown', onKeydown)
  await refreshAuth()
  if (auth.authenticated) load()
})
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))

watch(
  () => auth.authenticated,
  (authenticated) => {
    if (authenticated) load()
    else resetData()
  },
)
</script>

<template>
  <div
    v-if="!auth.ready"
    style="min-height: 100vh; display: grid; place-items: center; background: var(--bg)"
  >
    <div style="display: flex; flex-direction: column; align-items: center; gap: 14px">
      <div class="brand-mark" style="width: 44px; height: 44px; border-radius: 12px; font-size: 20px">
        ⚡
      </div>
      <div class="small muted">Loading calculation workspace…</div>
    </div>
  </div>

  <LoginView v-else-if="!auth.authenticated" />

  <div v-else class="app">
    <NavRail />
    <div class="main">
      <TopBar />
      <main class="content">
        <template v-if="ui.loading">
          <div class="skeleton" style="height: 74px" />
          <div class="grid cols-4">
            <div v-for="n in 4" :key="n" class="skeleton" style="height: 108px" />
          </div>
          <div class="skeleton" style="height: 320px" />
        </template>

        <div v-else-if="ui.loadError" class="card">
          <div class="card-body" style="text-align: center; padding: 42px 20px">
            <h3 style="margin-bottom: 6px">Cannot reach the calculation service</h3>
            <p class="muted small" style="margin-bottom: 16px">{{ ui.loadError }}</p>
            <button class="btn primary" @click="load()">Retry</button>
          </div>
        </div>

        <template v-else>
          <DashboardView v-if="ui.view === 'dashboard'" />
          <LoadListView v-else-if="ui.view === 'loadlist'" />
          <CableSizingView v-else-if="ui.view === 'cable'" />
          <CathodicView v-else-if="ui.view === 'cathodic'" />
          <MotorTableView v-else-if="ui.view === 'motors'" />
          <MethodView v-else />
        </template>
      </main>
    </div>

    <ItemDrawer v-if="ui.drawerId" />
    <CableDrawer v-if="ui.cableDrawerId" />
    <CathodicDrawer v-if="ui.cathodicDrawerId" />
    <SettingsDialog v-if="ui.settingsOpen" />
    <NewItemDialog v-if="ui.createOpen" />
    <Toasts />
  </div>
</template>
