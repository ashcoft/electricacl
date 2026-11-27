<script setup lang="ts">
import { computed, ref } from 'vue'
import CableDrawer from '../components/CableDrawer.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import Icon from '../components/Icon.vue'
import { blankCableCircuit, cableLabel, coresLabel } from '../lib/cable'
import { fmt } from '../lib/format'
import { api } from '../lib/api'
import { createCableCircuit, data, resetCableCircuits, ui } from '../store'
import type { CableRow } from '../types'

const state = computed(() => data.cable)
const search = ref('')
const confirmReset = ref(false)

const rows = computed<CableRow[]>(() => {
  const needle = search.value.trim().toLowerCase()
  const circuits = state.value?.circuits ?? []
  if (!needle) return circuits
  return circuits.filter((row) =>
    [row.tag, row.from_tag, row.to_tag, row.from_description, row.to_description, row.feeder_load, row.group]
      .filter(Boolean)
      .join(' ')
      .toLowerCase()
      .includes(needle),
  )
})

const stats = computed(() => state.value?.stats)
const totalVd = computed(() =>
  Math.max(0, ...(state.value?.circuits ?? []).map((row) => row.derived.vd_last_pct ?? 0)),
)

function checkClass(value: boolean | null): string {
  if (value === null) return 'pill neutral'
  return value ? 'pill continuous' : 'pill unassigned'
}

async function doReset() {
  confirmReset.value = false
  await resetCableCircuits()
}

async function newCircuit() {
  const created = await createCableCircuit(blankCableCircuit(state.value?.reference ?? []))
  if (created) ui.cableDrawerId = created.id
}
</script>

<template>
  <div style="display: flex; flex-direction: column; gap: 16px">
    <div class="page-head">
      <div>
        <div class="eyebrow">Appendix 2 · Cable sizing</div>
        <h1>Cable sizing calculation</h1>
        <p>
          {{ stats?.circuits ?? 0 }} circuits · AS/NZS 3008 · {{ stats?.groups ?? 0 }} series chains ·
          allowable drop per circuit
        </p>
      </div>
      <div class="spacer" />
      <div class="spread">
        <a class="btn" :href="api.cableCsvUrl" download title="Export the cable schedule as CSV">
          <Icon name="download" :size="14" /> CSV
        </a>
        <button class="btn" :disabled="ui.busy" @click="confirmReset = true">
          Restore source data
        </button>
        <button class="btn primary" :disabled="ui.busy" @click="newCircuit">
          <Icon name="plus" :size="14" /> New circuit
        </button>
      </div>
    </div>

    <div class="grid cols-4">
      <div class="card kpi">
        <div class="kpi-label">Circuits</div>
        <div class="kpi-value"><b>{{ stats?.circuits ?? 0 }}</b><span>items</span></div>
        <div class="kpi-foot">{{ stats?.groups ?? 0 }} series chains</div>
      </div>
      <div class="card kpi accent-green">
        <div class="kpi-label">Acceptable</div>
        <div class="kpi-value"><b>{{ stats?.acceptable ?? 0 }}</b><span>circuits</span></div>
        <div class="kpi-foot">all four checks pass</div>
      </div>
      <div class="card kpi accent-amber">
        <div class="kpi-label">Requires attention</div>
        <div class="kpi-value"><b>{{ stats?.failed ?? 0 }}</b><span>circuits</span></div>
        <div class="kpi-foot">ampacity, protection, drop or length</div>
      </div>
      <div class="card kpi accent-violet">
        <div class="kpi-label">Worst chain drop</div>
        <div class="kpi-value"><b>{{ fmt(totalVd, 2) }}</b><span>%</span></div>
        <div class="kpi-foot">{{ fmt(stats?.total_length_m ?? 0, 0) }} m of cable</div>
      </div>
    </div>

    <section class="card">
      <div class="card-head" style="flex-wrap: wrap; gap: 10px">
        <div class="search">
          <span class="icon"><Icon name="search" :size="13" /></span>
          <input v-model="search" class="input" placeholder="Search tag, panel, feeder load…" />
        </div>
        <div class="spacer" />
        <span class="pill neutral">{{ rows.length }} rows</span>
      </div>

      <div class="table-wrap" style="max-height: calc(100vh - 360px)">
        <table class="grid-table">
          <thead>
            <tr>
              <th class="num" style="width: 44px">SI</th>
              <th style="min-width: 150px">Cable tag</th>
              <th style="min-width: 210px">From → To</th>
              <th class="num" style="width: 74px">Load kW</th>
              <th class="num" style="width: 60px">V</th>
              <th style="width: 54px">Ph</th>
              <th class="num" style="width: 62px">L (m)</th>
              <th class="num" style="width: 52px">Runs</th>
              <th style="width: 118px">Cable</th>
              <th class="num" style="width: 62px">It (A)</th>
              <th class="num" style="width: 56px">CO</th>
              <th class="num" style="width: 70px">IZ (A)</th>
              <th class="num" style="width: 74px">IFLC (A)</th>
              <th class="num" style="width: 68px">Runs req</th>
              <th class="num" style="width: 66px">VD (V)</th>
              <th class="num" style="width: 62px">%VD</th>
              <th class="num" style="width: 70px">%VD last</th>
              <th class="num" style="width: 72px">Lmax (m)</th>
              <th style="width: 120px">Checks</th>
              <th style="width: 118px">Verdict</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="row in rows"
              :key="row.id"
              class="row-click"
              :class="{ selected: ui.cableDrawerId === row.id }"
              @click="ui.cableDrawerId = row.id"
            >
              <td class="num dim">{{ row.si_no ?? '–' }}</td>
              <td class="mono">{{ row.tag ?? '–' }}</td>
              <td>
                <div>{{ row.from_description ?? '—' }}</div>
                <div class="muted small">→ {{ row.to_description ?? '—' }}</div>
              </td>
              <td class="num">{{ fmt(row.load_kw, 2) }}</td>
              <td class="num dim">{{ fmt(row.voltage_v, 0) }}</td>
              <td class="dim small">{{ row.phases === 'three' ? '3ph' : '1ph' }}</td>
              <td class="num dim">{{ fmt(row.length_m, 0) }}</td>
              <td class="num dim">{{ row.runs_selected }}</td>
              <td class="mono small">{{ cableLabel(row.cores, row.size_mm2) }}</td>
              <td class="num dim">{{ fmt(row.derived.ampacity_a, 0) }}</td>
              <td class="num dim">{{ fmt(row.derived.derating, 2) }}</td>
              <td class="num">{{ fmt(row.derived.iz_a, 1) }}</td>
              <td class="num">{{ fmt(row.derived.iflc_a, 2) }}</td>
              <td class="num dim">{{ fmt(row.derived.runs_required, 2) }}</td>
              <td class="num">{{ fmt(row.derived.vd_v, 2) }}</td>
              <td class="num">{{ fmt(row.derived.vd_pct, 3) }}</td>
              <td class="num dim">
                {{ row.derived.vd_last_pct === null ? '–' : fmt(row.derived.vd_last_pct, 3) }}
              </td>
              <td class="num dim">
                {{ row.derived.max_length_m === null ? '–' : fmt(row.derived.max_length_m, 0) }}
              </td>
              <td>
                <span class="scenario-dots" style="gap: 4px">
                  <i
                    :class="{ on: row.derived.checks.ampacity }"
                    :title="`Ampacity: IZ ${fmt(row.derived.iz_a, 1)} A vs IFLC ${fmt(row.derived.iflc_a, 2)} A`"
                  />
                  <i
                    :class="{ on: row.derived.checks.protection }"
                    :title="`Protection: IZ vs IPR ${fmt(row.protection_rating_a, 0)} A`"
                  />
                  <i
                    v-if="row.derived.checks.voltage_drop !== null"
                    :class="{ on: row.derived.checks.voltage_drop }"
                    :title="`Voltage drop: ${fmt(row.derived.vd_last_pct ?? 0, 3)} % vs ${fmt(row.allowable_vd_pct ?? 0, 1)} %`"
                  />
                  <span
                    v-else
                    class="muted small"
                    style="width: 7px; text-align: center"
                    title="Cumulative drop is checked on the first circuit of the chain"
                    >–</span
                  >
                  <i
                    v-if="row.derived.checks.max_length !== null"
                    :class="{ on: row.derived.checks.max_length }"
                    :title="`Length: max ${fmt(row.derived.max_length_m ?? 0, 0)} m vs ${fmt(row.derived.chain_length_m ?? 0, 0)} m`"
                  />
                  <span
                    v-else
                    class="muted small"
                    style="width: 7px; text-align: center"
                    title="Maximum length is checked on the first circuit of the chain"
                    >–</span
                  >
                </span>
              </td>
              <td>
                <span class="pill" :class="row.derived.acceptable ? 'continuous' : 'unassigned'">
                  {{ row.derived.acceptable ? 'ACCEPTABLE' : 'CHECK' }}
                </span>
              </td>
            </tr>
            <tr v-if="!rows.length">
              <td colspan="20"><div class="empty">No circuit matches the search.</div></td>
            </tr>
          </tbody>
        </table>
      </div>
      <div class="card-body small muted" style="padding-top: 10px">
        Checks: ampacity (IZ ≥ IFLC) · protection (IZ &gt; IPR) · voltage drop (%VD last ≤ allowable)
        · maximum length (Lmax ≥ chain length). Orange = pass, grey = fail, – = checked on the
        first circuit of the chain only.
      </div>
    </section>

    <section class="card">
      <div class="card-head">
        <h3>Cable reference table</h3>
        <span class="sub">AS/NZS 3008 Tables 14 / 30 / 35</span>
        <div class="spacer" />
        <span class="pill neutral">{{ state?.reference.length ?? 0 }} sizes</span>
      </div>
      <div class="table-wrap">
        <table class="grid-table">
          <thead>
            <tr>
              <th>Cores</th>
              <th class="num">Size (mm²)</th>
              <th class="num">It (A)</th>
              <th class="num">R (Ω/km, 90 °C)</th>
              <th class="num">X (Ω/km)</th>
              <th>Source</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="entry in state?.reference ?? []" :key="`${entry.cores}-${entry.size_mm2}`">
              <td class="mono">{{ coresLabel(entry.cores) }}</td>
              <td class="num strong">{{ fmt(entry.size_mm2, entry.size_mm2 < 10 ? 1 : 0) }}</td>
              <td class="num">{{ fmt(entry.ampacity_a, 0) }}</td>
              <td class="num dim">{{ fmt(entry.resistance_ohm_km, 4) }}</td>
              <td class="num dim">{{ fmt(entry.reactance_ohm_km, 4) }}</td>
              <td class="muted small">{{ entry.source ?? '—' }}</td>
            </tr>
          </tbody>
        </table>
      </div>
      <div class="card-body small muted">
        Values used by the source workbook. Extend the table through the API
        (<span class="mono">PUT /api/cable/reference</span>) to add sizes for your own projects.
      </div>
    </section>

    <CableDrawer v-if="ui.cableDrawerId" />

    <ConfirmDialog
      v-if="confirmReset"
      title="Restore source cable data?"
      message="Every manual change to the cable schedule will be discarded and the 10 circuits from the source workbook will be restored."
      confirm-label="Restore"
      danger
      :busy="ui.busy"
      @confirm="doReset"
      @cancel="confirmReset = false"
    />
  </div>
</template>
