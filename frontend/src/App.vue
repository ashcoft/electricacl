<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import DashboardView from './views/DashboardView.vue'
import LoadListView from './views/LoadListView.vue'
import MethodView from './views/MethodView.vue'
import MotorTableView from './views/MotorTableView.vue'
import ItemDrawer from './components/ItemDrawer.vue'
import NavRail from './components/NavRail.vue'
import NewItemDialog from './components/NewItemDialog.vue'
import SettingsDialog from './components/SettingsDialog.vue'
import Toasts from './components/Toasts.vue'
import TopBar from './components/TopBar.vue'
import { load, ui } from './store'

function onKeydown(event: KeyboardEvent) {
  const target = event.target as HTMLElement | null
  const typing =
    target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT')

  if (event.key === 'Escape') {
    if (ui.createOpen) ui.createOpen = false
    else if (ui.settingsOpen) ui.settingsOpen = false
    else if (ui.drawerId) ui.drawerId = null
    return
  }
  if (typing) return
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

onMounted(() => {
  load()
  window.addEventListener('keydown', onKeydown)
})
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <div class="app">
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
          <MotorTableView v-else-if="ui.view === 'motors'" />
          <MethodView v-else />
        </template>
      </main>
    </div>

    <ItemDrawer v-if="ui.drawerId" />
    <SettingsDialog v-if="ui.settingsOpen" />
    <NewItemDialog v-if="ui.createOpen" />
    <Toasts />
  </div>
</template>
