<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { blankItem, computeDerived } from '../lib/calc'
import { fmt } from '../lib/format'
import { createItem, data, ui } from '../store'
import { CLASSES, DUTIES, SCENARIOS, type ClassKey, type LoadItem, type ScenarioKey } from '../types'

const draft = reactive<LoadItem>(blankItem())
const busy = ref(false)

const motorTable = computed(() => data.state?.motor_table ?? [])
const derived = computed(() => computeDerived(draft, motorTable.value))

function toggleScenario(key: ScenarioKey) {
  const set = new Set(draft.scenarios)
  if (set.has(key)) set.delete(key)
  else set.add(key)
  draft.scenarios = SCENARIOS.filter((entry) => set.has(entry.key)).map((entry) => entry.key)
}

async function submit() {
  if (busy.value) return
  busy.value = true
  const created = await createItem({ ...draft })
  busy.value = false
  if (created) {
    ui.createOpen = false
    ui.drawerId = created.id
    ui.view = 'loadlist'
  }
}
</script>

<template>
  <div class="modal-backdrop" @click.self="ui.createOpen = false">
    <div class="modal">
      <div class="modal-head">
        <h2>New load item</h2>
        <div class="spacer" style="margin-left: auto" />
        <button class="btn icon" @click="ui.createOpen = false">✕</button>
      </div>
      <div class="modal-body">
        <div class="form-grid">
          <div class="field">
            <label>Tag number</label>
            <input v-model="draft.tag" class="input" placeholder="F-000-X-0001" autofocus />
          </div>
          <div class="field">
            <label>Service duty</label>
            <select v-model="draft.duty" class="select">
              <option :value="null">Unassigned</option>
              <option v-for="entry in DUTIES" :key="entry.key" :value="entry.key">
                {{ entry.label }}
              </option>
            </select>
          </div>
          <div class="field span-2">
            <label>Description</label>
            <input v-model="draft.description" class="input" placeholder="Consumer description" />
          </div>
          <div class="field">
            <label>Equipment class</label>
            <div class="segmented">
              <button
                v-for="entry in CLASSES"
                :key="entry.key"
                :class="{ active: draft.equipment_class === entry.key }"
                @click="draft.equipment_class = entry.key as ClassKey"
              >
                {{ entry.label }}
              </button>
            </div>
          </div>
          <div class="field">
            <label>Area</label>
            <select v-model="draft.area" class="select">
              <option :value="null">—</option>
              <option v-for="area in data.state?.options.areas ?? []" :key="area" :value="area">
                {{ area }}
              </option>
            </select>
          </div>
          <div class="field">
            <label>Rated voltage (kV)</label>
            <input v-model.number="draft.voltage_kv" class="input num" type="number" step="0.01" min="0" />
          </div>
          <div class="field">
            <label>Absorbed power (kW)</label>
            <input v-model.number="draft.absorbed_kw" class="input num" type="number" step="0.1" min="0" />
          </div>
          <div class="field">
            <label>Rated power (kW)</label>
            <input v-model.number="draft.rated_kw" class="input num" type="number" step="0.1" min="0" />
          </div>
          <div class="field">
            <label>Consumed load</label>
            <div class="input num" style="background: var(--brand-tint); color: var(--brand-dark); font-weight: 600">
              {{ fmt(derived.consumed_kw, 2) }} kW · {{ fmt(derived.flc_a, 1) }} A
            </div>
          </div>
        </div>

        <div>
          <div class="section-title">Operating scenarios</div>
          <div class="chip-row">
            <button
              v-for="entry in SCENARIOS"
              :key="entry.key"
              class="chip"
              :class="{ on: draft.scenarios.includes(entry.key) }"
              @click="toggleScenario(entry.key)"
            >
              {{ entry.label }}
            </button>
          </div>
        </div>
      </div>
      <div class="modal-foot">
        <button class="btn" @click="ui.createOpen = false">Cancel</button>
        <button class="btn primary" :disabled="busy" @click="submit">
          {{ busy ? 'Adding…' : 'Add item' }}
        </button>
      </div>
    </div>
  </div>
</template>
