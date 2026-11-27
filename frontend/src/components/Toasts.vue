<script setup lang="ts">
import { ui } from '../store'
</script>

<template>
  <div class="toasts">
    <div v-for="entry in ui.toasts" :key="entry.id" class="toast" :class="entry.kind">
      <span>{{ entry.kind === 'error' ? '✕' : entry.kind === 'success' ? '✓' : 'ℹ' }}</span>
      <span>{{ entry.message }}</span>
    </div>
  </div>
</template>
