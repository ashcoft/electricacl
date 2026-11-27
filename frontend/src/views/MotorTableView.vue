<script setup lang="ts">
import { computed, ref } from 'vue'
import { fmt, fmtPct } from '../lib/format'
import { data } from '../store'

const search = ref('')

const usage = computed(() => {
  const counts = new Map<number, number>()
  for (const item of data.state?.items ?? []) {
    if (item.equipment_class !== 'motor') continue
    counts.set(item.rated_kw, (counts.get(item.rated_kw) ?? 0) + 1)
  }
  return counts
})

const rows = computed(() => {
  const needle = search.value.trim().toLowerCase()
  return (data.state?.motor_table ?? []).filter((row) => {
    if (!needle) return true
    return String(row.rated_kw).includes(needle)
  })
})
</script>

<template>
  <div style="display: flex; flex-direction: column; gap: 14px">
    <div class="page-head">
      <div>
        <div class="eyebrow">Appendix 11</div>
        <h1>Motor efficiency &amp; power factor</h1>
        <p>
          Reference curve used for every motor consumer. Efficiency and power factor are interpolated at
          the actual load factor.
        </p>
      </div>
      <div class="spacer" />
      <div class="search">
        <span class="icon">⌕</span>
        <input v-model="search" class="input" placeholder="Filter by rating (kW)" />
      </div>
    </div>

    <section class="card">
      <div class="card-head">
        <h3>Reference table</h3>
        <span class="sub">{{ rows.length }} ratings</span>
        <div class="spacer" />
        <span class="pill neutral">η and cos φ at 100 / 75 / 50 % load</span>
      </div>
      <div class="table-wrap" style="max-height: calc(100vh - 290px)">
        <table class="grid-table">
          <thead>
            <tr>
              <th class="num">Rated (kW)</th>
              <th class="num">Ist / In</th>
              <th class="num">η 100 %</th>
              <th class="num">η 75 %</th>
              <th class="num">η 50 %</th>
              <th class="num">cos φ 100 %</th>
              <th class="num">cos φ 75 %</th>
              <th class="num">cos φ 50 %</th>
              <th class="num">Consumers</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in rows" :key="row.rated_kw">
              <td class="num strong">{{ fmt(row.rated_kw, row.rated_kw < 10 ? 2 : 0) }}</td>
              <td class="num dim">{{ fmt(row.starting_ratio, 1) }}</td>
              <td class="num">{{ fmtPct(row.eff_full, 1) }}</td>
              <td class="num dim">{{ fmtPct(row.eff_75, 1) }}</td>
              <td class="num dim">{{ fmtPct(row.eff_50, 1) }}</td>
              <td class="num">{{ fmt(row.pf_full, 3) }}</td>
              <td class="num dim">{{ fmt(row.pf_75, 3) }}</td>
              <td class="num dim">{{ fmt(row.pf_50, 3) }}</td>
              <td class="num">
                <span v-if="usage.get(row.rated_kw)" class="pill neutral">{{ usage.get(row.rated_kw) }}</span>
                <span v-else class="muted">–</span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>
  </div>
</template>
