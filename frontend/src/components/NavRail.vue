<script setup lang="ts">
import { computed } from 'vue'
import { data, ui, type ViewKey } from '../store'

const stats = computed(() => data.state?.stats)

const views: { key: ViewKey; label: string; icon: string }[] = [
  { key: 'dashboard', label: 'Dashboard', icon: '▤' },
  { key: 'loadlist', label: 'Load list', icon: '☰' },
  { key: 'motors', label: 'Motor data', icon: '⏣' },
  { key: 'method', label: 'Method & basis', icon: 'ƒ' },
]

const upcoming = [
  { idx: '02', label: 'Cable sizing' },
  { idx: '03', label: 'Cathodic protection' },
  { idx: '04', label: 'Transformer sizing' },
]
</script>

<template>
  <aside class="rail">
    <div class="brand">
      <div class="brand-mark">⚡</div>
      <div class="brand-text">
        <strong>Electrical</strong>
        <span>Calculation suite</span>
      </div>
    </div>

    <div class="nav-group">
      <div class="nav-title">Calculations</div>
      <button class="nav-item active" @click="ui.view = 'dashboard'">
        <span class="idx">01</span>
        <span>Load list</span>
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
        <span class="idx">{{ entry.icon }}</span>
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
