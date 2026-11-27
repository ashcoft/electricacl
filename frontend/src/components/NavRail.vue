<script setup lang="ts">
import { computed } from 'vue'
import { data, ui, type ViewKey } from '../store'
import Icon from './Icon.vue'

const stats = computed(() => data.state?.stats)

const views: {
  key: ViewKey
  label: string
  icon: 'dashboard' | 'list' | 'motor' | 'formula'
}[] = [
  { key: 'dashboard', label: 'Dashboard', icon: 'dashboard' },
  { key: 'loadlist', label: 'Load list', icon: 'list' },
  { key: 'motors', label: 'Motor data', icon: 'motor' },
  { key: 'method', label: 'Method & basis', icon: 'formula' },
]

const upcoming = [{ idx: '04', label: 'Transformer sizing' }]
</script>

<template>
  <aside class="rail">
    <div class="brand">
      <div class="brand-mark">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
          <path d="M13.5 2 4 13.5h5.5L9 22l9.5-11.5H13z" />
        </svg>
      </div>
      <div class="brand-text">
        <strong>Electrical</strong>
        <span>Calculation suite</span>
      </div>
    </div>

    <div class="nav-group">
      <div class="nav-title">Calculations</div>
      <button
        class="nav-item"
        :class="{ active: ui.view === 'dashboard' || ui.view === 'loadlist' }"
        @click="ui.view = 'dashboard'"
      >
        <span class="idx">01</span>
        <span>Load list</span>
      </button>
      <button
        class="nav-item"
        :class="{ active: ui.view === 'cable' }"
        @click="ui.view = 'cable'"
      >
        <span class="idx">02</span>
        <span>Cable sizing</span>
      </button>
      <button
        class="nav-item"
        :class="{ active: ui.view === 'cathodic' }"
        @click="ui.view = 'cathodic'"
      >
        <span class="idx">03</span>
        <span>Cathodic protection</span>
      </button>
      <button
        v-for="entry in upcoming"
        :key="entry.idx"
        class="nav-item disabled"
        disabled
        title="Planned - not part of this release"
      >
        <span class="idx">{{ entry.idx }}</span>
        <span>{{ entry.label }}</span>
        <span class="soon">soon</span>
      </button>
    </div>

    <div class="nav-group">
      <div class="nav-title">Views</div>
      <button
        v-for="entry in views"
        :key="entry.key"
        class="nav-item"
        :class="{ active: ui.view === entry.key }"
        @click="ui.view = entry.key"
      >
        <span class="idx"><Icon :name="entry.icon" :size="14" /></span>
        <span>{{ entry.label }}</span>
      </button>
    </div>

    <div class="rail-foot">
      <div><b>{{ stats?.items ?? '—' }}</b> load items</div>
      <div><b>{{ stats?.motor_table_rows ?? '—' }}</b> motor reference rows</div>
      <div class="mono">Rev. {{ data.state?.project.revision ?? '—' }}</div>
    </div>
  </aside>
</template>
