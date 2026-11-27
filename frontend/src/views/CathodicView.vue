<script setup lang="ts">
import { computed, ref } from 'vue'
import CathodicDrawer from '../components/CathodicDrawer.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import Icon from '../components/Icon.vue'
import { api } from '../lib/api'
import { blankCathodicItem, methodLabel, scopeLabel } from '../lib/cathodic'
import { fmt } from '../lib/format'
import { createCathodicItem, data, resetCathodicItems, ui } from '../store'
import type { CathodicRow } from '../types'

const state = computed(() => data.cathodic)
const search = ref('')
const confirmReset = ref(false)

const rows = computed<CathodicRow[]>(() => {
  const needle = search.value.trim().toLowerCase()
  const items = state.value?.items ?? []
  if (!needle) return items
  return items.filter((row) =>
    [row.tag, row.description, row.source_sheet, row.structure, row.orientation, row.method, row.anode_type]
      .filter(Boolean)
      .join(' ')
      .toLowerCase()
      .includes(needle),
  )
})

const stats = computed(() => state.value?.stats)

async function doReset() {
  confirmReset.value = false
  await resetCathodicItems()
}

async function newItem() {
  const created = await createCathodicItem(blankCathodicItem())
  if (created) ui.cathodicDrawerId = created.id
}
</script>

<template>
  <div style="display: flex; flex-direction: column; gap: 16px">
    <div class="page-head">
      <div>
        <div class="eyebrow">Calculation 03 · Cathodic protection</div>
        <h1>Cathodic protection</h1>
        <p>
          {{ stats?.items ?? 0 }} protected items · {{ stats?.sacp ?? 0 }} SACP ·
          {{ stats?.iccp ?? 0 }} ICCP · DNV RP B401 (2005) / GS EP COR 201
        </p>
      </div>
      <div class="spacer" />
      <div class="spread">
        <a class="btn" :href="api.cathodicCsvUrl" download title="Export the cathodic schedule as CSV">
          <Icon name="download" :size="14" /> CSV
        </a>
        <button class="btn" :disabled="ui.busy" @click="confirmReset = true">
          Restore source data
        </button>
        <button class="btn primary" :disabled="ui.busy" @click="newItem">
          <Icon name="plus" :size="14" /> New item
        </button>
      </div>
    </div>

    <div class="grid cols-4">
      <div class="card kpi">
        <div class="kpi-label">Protected items</div>
        <div class="kpi-value"><b>{{ stats?.items ?? 0 }}</b><span>items</span></div>
        <div class="kpi-foot">{{ stats?.sacp ?? 0 }} SACP · {{ stats?.iccp ?? 0 }} ICCP</div>
      </div>
      <div class="card kpi accent-green">
        <div class="kpi-label">Acceptable</div>
        <div class="kpi-value"><b>{{ stats?.acceptable ?? 0 }}</b><span>items</span></div>
        <div class="kpi-foot">capacity, current and anode count pass</div>
      </div>
      <div class="card kpi accent-amber">
        <div class="kpi-label">Requires attention</div>
        <div class="kpi-value"><b>{{ stats?.failed ?? 0 }}</b><span>items</span></div>
        <div class="kpi-foot">{{ stats?.warnings ?? 0 }} findings</div>
      </div>
      <div class="card kpi accent-violet">
        <div class="kpi-label">Protection current</div>
        <div class="kpi-value"><b>{{ fmt(stats?.total_protection_current_a ?? 0, 1) }}</b><span>A</span></div>
        <div class="kpi-foot">{{ fmt(stats?.total_anode_mass_kg ?? 0, 0) }} kg of anode material</div>
      </div>
    </div>

    <section class="card">
      <div class="card-head" style="flex-wrap: wrap; gap: 10px">
        <div class="search">
          <span class="icon"><Icon name="search" :size="13" /></span>
          <input v-model="search" class="input" placeholder="Search tag, description, structure, anode…" />
        </div>
        <div class="spacer" />
        <span class="pill neutral">{{ rows.length }} rows</span>
      </div>

      <div class="table-wrap" style="max-height: calc(100vh - 360px)">
        <table class="grid-table">
          <thead>
            <tr>
              <th style="min-width: 190px">Tag / description</th>
              <th style="min-width: 190px">Scope</th>
              <th style="width: 64px">Method</th>
              <th class="num" style="width: 86px">Cdu (A/m²)</th>
              <th class="num" style="width: 82px">SA (m²)</th>
              <th class="num" style="width: 78px">Ip (A)</th>
              <th style="min-width: 130px">Anode</th>
              <th class="num" style="width: 82px">Mass (kg)</th>
              <th class="num" style="width: 70px">Req / sel</th>
              <th class="num" style="width: 64px">Life (y)</th>
              <th style="width: 96px">Checks</th>
              <th style="width: 110px">Verdict</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="row in rows"
              :key="row.id"
              class="row-click"
              :class="{ selected: ui.cathodicDrawerId === row.id }"
              @click="ui.cathodicDrawerId = row.id"
            >
              <td>
                <div class="mono">{{ row.tag ?? '–' }}</div>
                <div class="muted small">
                  {{
                    row.description && row.description !== row.tag
                      ? row.description
                      : row.source_sheet && row.source_sheet !== row.tag
                        ? row.source_sheet
                        : '—'
                  }}
                </div>
              </td>
              <td>
                <div class="small">{{ scopeLabel(row) }}</div>
                <div class="muted small">
                  D {{ fmt(row.diameter_m, 2) }} m
                  <template v-if="row.structure === 'tank' || row.structure === 'vessel'">
                    · L {{ fmt(row.length_m, 2) }} m · LH {{ fmt(row.liquid_height_m, 2) }} m
                  </template>
                </div>
              </td>
              <td><span class="pill neutral">{{ methodLabel(row.method) }}</span></td>
              <td class="num">{{ fmt(row.derived.current_density_a_m2, 5) }}</td>
              <td class="num">{{ fmt(row.derived.exposed_area_m2, 1) }}</td>
              <td class="num">{{ fmt(row.derived.protection_current_a, 2) }}</td>
              <td class="small">
                <div>{{ row.anode_type ?? '—' }}</div>
                <div class="muted">
                  {{ fmt(row.anode_weight_kg, 1) }} kg
                  <template v-if="row.method === 'iccp'">
                    · {{ fmt(row.derived.rectifier_kva ?? 0, 2) }} kVA
                  </template>
                </div>
              </td>
              <td class="num">{{ row.derived.anode_mass_kg === null ? '–' : fmt(row.derived.anode_mass_kg, 0) }}</td>
              <td class="num">
                {{ row.derived.anodes_required }} / <b>{{ row.derived.anodes_selected }}</b>
              </td>
              <td class="num dim">{{ row.derived.life_years === null ? '–' : fmt(row.derived.life_years, 1) }}</td>
              <td>
                <span class="scenario-dots" style="gap: 4px">
                  <i :class="{ on: row.derived.checks.capacity }" title="Anode capacity is positive" />
                  <i :class="{ on: row.derived.checks.current_density }" title="Design current density is positive" />
                  <i :class="{ on: row.derived.checks.anode_count }" title="Selected anodes cover the requirement" />
                  <i :class="{ on: row.derived.checks.design_life }" title="Selected anodes reach the design life" />
                </span>
              </td>
              <td>
                <span class="pill" :class="row.derived.acceptable ? 'continuous' : 'unassigned'">
                  {{ row.derived.acceptable ? 'ACCEPTABLE' : 'CHECK' }}
                </span>
              </td>
            </tr>
            <tr v-if="!rows.length">
              <td colspan="12"><div class="empty">No item matches the search.</div></td>
            </tr>
          </tbody>
        </table>
      </div>
      <div class="card-body small muted" style="padding-top: 10px">
        SACP sizes the anode mass from the protection current, design life and anode capacity; ICCP
        sizes the groundbed and rectifier. Checks: capacity · current density · anode count · design
        life. The 23 SACP items reproduce the source workbook to 1 × 10⁻⁶.
      </div>
    </section>

    <CathodicDrawer v-if="ui.cathodicDrawerId" />

    <ConfirmDialog
      v-if="confirmReset"
      title="Restore source cathodic data?"
      message="Every manual change to the cathodic schedule will be discarded and the 23 items from the source workbook will be restored."
      confirm-label="Restore"
      danger
      :busy="ui.busy"
      @confirm="doReset"
      @cancel="confirmReset = false"
    />
  </div>
</template>
