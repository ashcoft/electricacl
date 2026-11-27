<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { computeCableDerived, coresLabel } from '../lib/cable'
import { fmt } from '../lib/format'
import {
  cableDrawerItem,
  data,
  deleteCableCircuit,
  duplicateCableCircuit,
  saveCableCircuit,
  ui,
} from '../store'
import { CABLE_CORES, PHASES, type CableCircuit, type CableRow } from '../types'
import ConfirmDialog from './ConfirmDialog.vue'

const draft = ref<CableCircuit | null>(null)
const confirmDelete = ref(false)
const saving = ref(false)

const reference = computed(() => data.cable?.reference ?? [])

function strip(row: CableRow): CableCircuit {
  const copy = JSON.parse(JSON.stringify(row)) as Record<string, unknown>
  delete copy.derived
  return copy as unknown as CableCircuit
}

watch(
  () => cableDrawerItem.value,
  (row) => {
    if (row) draft.value = strip(row)
  },
  { immediate: true },
)

const derived = computed(() =>
  draft.value ? computeCableDerived(draft.value, reference.value) : null,
)

const dirty = computed(() => {
  if (!draft.value || !cableDrawerItem.value) return false
  return JSON.stringify(draft.value) !== JSON.stringify(strip(cableDrawerItem.value))
})

const sizeOptions = computed(() => {
  if (!draft.value) return []
  return reference.value
    .filter((entry) => entry.cores === draft.value?.cores)
    .map((entry) => entry.size_mm2)
})

function onCoresChange(event: Event) {
  if (!draft.value) return
  const cores = (event.target as HTMLSelectElement).value as CableCircuit['cores']
  draft.value.cores = cores
  const sizes = reference.value.filter((entry) => entry.cores === cores)
  if (sizes.length && !sizes.some((entry) => entry.size_mm2 === draft.value?.size_mm2)) {
    draft.value.size_mm2 = sizes[0].size_mm2
  }
}

async function save() {
  if (!draft.value || saving.value) return
  saving.value = true
  const result = await saveCableCircuit(draft.value)
  saving.value = false
  if (result && cableDrawerItem.value) draft.value = strip(cableDrawerItem.value)
}

async function duplicate() {
  if (!cableDrawerItem.value) return
  const copy = await duplicateCableCircuit(cableDrawerItem.value.id)
  if (copy) ui.cableDrawerId = copy.id
}

async function remove() {
  if (!cableDrawerItem.value) return
  const id = cableDrawerItem.value.id
  confirmDelete.value = false
  await deleteCableCircuit(id)
  ui.cableDrawerId = null
}
</script>

<template>
  <div v-if="draft && derived">
    <div class="drawer-backdrop" @click="ui.cableDrawerId = null" />
    <aside class="drawer">
      <div class="drawer-head">
        <div style="min-width: 0">
          <h2>{{ draft.tag || 'New cable circuit' }}</h2>
          <div class="tag">
            {{ coresLabel(draft.cores) }} x {{ draft.size_mm2 }} mm² ·
            {{ draft.phases === 'three' ? '3-phase' : '1-phase' }} · {{ fmt(draft.voltage_v, 0) }} V
          </div>
        </div>
        <div class="spacer" style="margin-left: auto" />
        <button class="btn icon" title="Close (Esc)" @click="ui.cableDrawerId = null">
          ✕
        </button>
      </div>

      <div class="drawer-body">
        <div class="computed">
          <div class="big">
            <b>{{ fmt(derived.iz_a, 1) }}</b>
            <span>A derated capacity (IZ)</span>
            <span style="margin-left: auto" class="mono">{{ fmt(derived.iflc_a, 2) }} A IFLC</span>
          </div>
          <div class="small muted">
            {{ fmt(derived.ampacity_a, 0) }} A table × {{ fmt(derived.derating, 2) }} derating ·
            voltage drop {{ fmt(derived.vd_v, 2) }} V ({{ fmt(derived.vd_pct, 3) }} %)
          </div>

          <div class="divider" style="margin: 12px 0" />

          <dl class="kv">
            <dt>Load factor / runs required</dt>
            <dd>{{ fmt(derived.runs_required, 3) }} → {{ draft.runs_selected }} selected</dd>
            <dt>Voltage at end of run</dt>
            <dd>{{ fmt(derived.voltage_at_end_v, 1) }} V</dd>
            <dt>Cumulative drop (chain head)</dt>
            <dd>{{ derived.vd_last_pct === null ? '–' : `${fmt(derived.vd_last_pct, 3)} %` }}</dd>
            <dt>Maximum length</dt>
            <dd>{{ derived.max_length_m === null ? '–' : `${fmt(derived.max_length_m, 0)} m` }}</dd>
          </dl>

          <div class="formula" style="margin-top: 12px">
            IFLC = kW × 1000 / ({{ draft.phases === 'three' ? '1.732' : '1' }} × V × cos φ) =
            {{ fmt(derived.iflc_a, 2) }} A<br />
            IZ = It × Cg × Ca = {{ fmt(derived.ampacity_a, 0) }} × {{ fmt(draft.grouping_factor, 2) }}
            × {{ fmt(draft.temperature_factor, 2) }} = {{ fmt(derived.iz_a, 1) }} A<br />
            VD = {{ draft.phases === 'three' ? '1.732' : '2' }} × IFLC × (L/1000) / runs × (R cos φ + X sin φ) =
            {{ fmt(derived.vd_v, 2) }} V
          </div>

          <div style="display: flex; gap: 6px; flex-wrap: wrap; margin-top: 12px">
            <span class="pill" :class="derived.checks.ampacity ? 'continuous' : 'unassigned'">
              Ampacity
            </span>
            <span class="pill" :class="derived.checks.protection ? 'continuous' : 'unassigned'">
              Protection
            </span>
            <span
              class="pill"
              :class="derived.checks.voltage_drop === null ? 'neutral' : derived.checks.voltage_drop ? 'continuous' : 'unassigned'"
            >
              Voltage drop
            </span>
            <span
              class="pill"
              :class="derived.checks.max_length === null ? 'neutral' : derived.checks.max_length ? 'continuous' : 'unassigned'"
            >
              Max length
            </span>
          </div>
        </div>

        <div v-if="derived.warnings.length" class="warning-list">
          <div v-for="warning in derived.warnings" :key="warning.code" class="warning-item">
            <span>!</span>
            <span>{{ warning.message }}</span>
          </div>
        </div>

        <div>
          <div class="section-title">Identification</div>
          <div class="form-grid">
            <div class="field">
              <label>Cable tag</label>
              <input v-model="draft.tag" class="input" placeholder="3000-NLP-2001-P01" />
            </div>
            <div class="field">
              <label>SI no.</label>
              <input v-model.number="draft.si_no" class="input num" type="number" min="0" />
            </div>
            <div class="field">
              <label>Series chain</label>
              <input v-model="draft.group" class="input" placeholder="NLP-2001 (blank = own chain)" />
            </div>
            <div class="field">
              <label>Feeder load</label>
              <input v-model="draft.feeder_load" class="input" placeholder="LIGHTING / SMALL POWER / PANEL" />
            </div>
            <div class="field">
              <label>From (description)</label>
              <input v-model="draft.from_description" class="input" />
            </div>
            <div class="field">
              <label>From (tag)</label>
              <input v-model="draft.from_tag" class="input" />
            </div>
            <div class="field">
              <label>To (description)</label>
              <input v-model="draft.to_description" class="input" />
            </div>
            <div class="field">
              <label>To (tag)</label>
              <input v-model="draft.to_tag" class="input" />
            </div>
          </div>
        </div>

        <div>
          <div class="section-title">Load</div>
          <div class="form-grid">
            <div class="field">
              <label>Load (kW)</label>
              <input v-model.number="draft.load_kw" class="input num" type="number" step="0.01" min="0" />
            </div>
            <div class="field">
              <label>Power factor</label>
              <input v-model.number="draft.power_factor" class="input num" type="number" step="0.01" min="0.01" max="1" />
            </div>
            <div class="field">
              <label>Voltage (V)</label>
              <input v-model.number="draft.voltage_v" class="input num" type="number" step="1" min="1" />
            </div>
            <div class="field">
              <label>Phases</label>
              <select v-model="draft.phases" class="select">
                <option v-for="entry in PHASES" :key="entry.key" :value="entry.key">
                  {{ entry.label }}
                </option>
              </select>
            </div>
            <div class="field">
              <label>Length (m)</label>
              <input v-model.number="draft.length_m" class="input num" type="number" step="1" min="0" />
            </div>
            <div class="field">
              <label>Runs selected</label>
              <input v-model.number="draft.runs_selected" class="input num" type="number" step="1" min="1" />
            </div>
          </div>
        </div>

        <div>
          <div class="section-title">Cable</div>
          <div class="form-grid">
            <div class="field">
              <label>Cores</label>
              <select :value="draft.cores" class="select" @change="onCoresChange">
                <option v-for="entry in CABLE_CORES" :key="entry.key" :value="entry.key">
                  {{ entry.label }}
                </option>
              </select>
            </div>
            <div class="field">
              <label>Size (mm²)</label>
              <select v-model.number="draft.size_mm2" class="select">
                <option v-for="size in sizeOptions" :key="size" :value="size">{{ size }}</option>
              </select>
            </div>
            <div class="field">
              <label>Grouping factor Cg</label>
              <input v-model.number="draft.grouping_factor" class="input num" type="number" step="0.01" min="0.01" max="1.5" />
            </div>
            <div class="field">
              <label>Temperature factor Ca</label>
              <input v-model.number="draft.temperature_factor" class="input num" type="number" step="0.01" min="0.01" max="1.5" />
            </div>
            <div class="field">
              <label>Voltage grade</label>
              <input v-model="draft.voltage_grade" class="input" placeholder="0.6/1kV" />
            </div>
            <div class="field">
              <label>Cable type</label>
              <input v-model="draft.cable_type" class="input" placeholder="CU/XLPE/LSZH - FRT" />
            </div>
          </div>
        </div>

        <div>
          <div class="section-title">Protection &amp; limits</div>
          <div class="form-grid">
            <div class="field">
              <label>Protection setting IPR (A)</label>
              <input v-model.number="draft.protection_rating_a" class="input num" type="number" step="1" min="0" />
            </div>
            <div class="field">
              <label>Breaker rating (A)</label>
              <input v-model.number="draft.breaker_rating_a" class="input num" type="number" step="1" min="0" />
            </div>
            <div class="field">
              <label>Breaker type</label>
              <input v-model="draft.breaker_type" class="input" placeholder="MCB / MCCB" />
            </div>
            <div class="field">
              <label>Allowable drop (%)</label>
              <input v-model.number="draft.allowable_vd_pct" class="input num" type="number" step="0.1" min="0" />
              <div class="small muted">Chain head only; leave blank for downstream circuits.</div>
            </div>
          </div>
        </div>

        <div>
          <div class="section-title">Remarks</div>
          <textarea v-model="draft.remark" class="input" placeholder="Basis of the data, references, notes…" />
        </div>
      </div>

      <div class="drawer-foot">
        <button class="btn danger" @click="confirmDelete = true">Delete</button>
        <button class="btn" @click="duplicate">Duplicate</button>
        <div class="spacer" style="margin-left: auto" />
        <button class="btn" :class="{ primary: dirty }" :disabled="!dirty || saving" @click="save">
          {{ saving ? 'Saving…' : dirty ? 'Save changes' : '✓ Saved' }}
        </button>
      </div>
    </aside>

    <ConfirmDialog
      v-if="confirmDelete"
      title="Delete this circuit?"
      :message="`${draft.tag || 'This circuit'} will be removed from the cable schedule.`"
      confirm-label="Delete"
      danger
      @confirm="remove"
      @cancel="confirmDelete = false"
    />
  </div>
</template>
