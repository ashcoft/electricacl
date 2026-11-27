<script setup lang="ts">
import { reactive, ref } from 'vue'
import { data, saveSettings, ui } from '../store'

const draft = reactive({
  contingency_pct: data.state?.settings.contingency_pct ?? 20,
  intermittent_factor_pct: data.state?.settings.intermittent_factor_pct ?? 30,
  standby_factor_pct: data.state?.settings.standby_factor_pct ?? 10,
})
const busy = ref(false)

function defaults() {
  draft.contingency_pct = 20
  draft.intermittent_factor_pct = 30
  draft.standby_factor_pct = 10
}

async function submit() {
  if (busy.value) return
  busy.value = true
  const result = await saveSettings({ ...draft })
  busy.value = false
  if (result) ui.settingsOpen = false
}
</script>

<template>
  <div class="modal-backdrop" @click.self="ui.settingsOpen = false">
    <div class="modal" style="width: min(480px, 100%)">
      <div class="modal-head">
        <h2>Calculation settings</h2>
        <div class="spacer" style="margin-left: auto" />
        <button class="btn icon" @click="ui.settingsOpen = false">✕</button>
      </div>
      <div class="modal-body">
        <div class="field">
          <label>Spare contingency (%)</label>
          <input v-model.number="draft.contingency_pct" class="input num" type="number" min="0" max="200" step="1" />
          <div class="small muted">Applied to the peak load: demand = peak × (1 + contingency).</div>
        </div>
        <div class="field">
          <label>Intermittent diversity (%)</label>
          <input
            v-model.number="draft.intermittent_factor_pct"
            class="input num"
            type="number"
            min="0"
            max="100"
            step="1"
          />
          <div class="small muted">
            Applied to the intermittent total; the largest single intermittent load is used when it is
            bigger.
          </div>
        </div>
        <div class="field">
          <label>Stand-by diversity (%)</label>
          <input
            v-model.number="draft.standby_factor_pct"
            class="input num"
            type="number"
            min="0"
            max="100"
            step="1"
          />
          <div class="small muted">
            Applied to the stand-by total; the largest single stand-by load is used when it is bigger.
          </div>
        </div>
        <div class="formula">
          Peak = continuous + intermittent × {{ draft.intermittent_factor_pct }} % + max(stand-by ×
          {{ draft.standby_factor_pct }} %, largest stand-by)<br />
          Demand = peak × (1 + {{ draft.contingency_pct }} %)
        </div>
      </div>
      <div class="modal-foot">
        <button class="btn" style="margin-right: auto" @click="defaults">Reset defaults</button>
        <button class="btn" @click="ui.settingsOpen = false">Cancel</button>
        <button class="btn primary" :disabled="busy" @click="submit">
          {{ busy ? 'Saving…' : 'Apply' }}
        </button>
      </div>
    </div>
  </div>
</template>
