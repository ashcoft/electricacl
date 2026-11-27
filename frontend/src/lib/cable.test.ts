import { describe, expect, it } from 'vitest'
import { blankCableCircuit, cableLabel, computeCableDerived } from './cable'
import type { CableCircuit, CableReference } from '../types'

const reference: CableReference[] = [
  {
    cores: 'two_core_earth',
    size_mm2: 2.5,
    ampacity_a: 26,
    resistance_ohm_km: 9.45,
    reactance_ohm_km: 0.0988,
    source: null,
  },
  {
    cores: 'four_core_earth',
    size_mm2: 35,
    ampacity_a: 137,
    resistance_ohm_km: 0.669,
    reactance_ohm_km: 0.0786,
    source: null,
  },
]

function circuit(overrides: Partial<CableCircuit>): CableCircuit {
  return { ...blankCableCircuit(reference), ...overrides }
}

describe('computeCableDerived — mirrors the Rust engine', () => {
  it('reproduces the lighting circuit of the workbook', () => {
    // SI 2: 0.36 kW at 230 V, pf 0.9, 50 m, 2.5 mm2, Cg 0.66.
    const derived = computeCableDerived(
      circuit({ load_kw: 0.36, length_m: 50, grouping_factor: 0.66, allowable_vd_pct: 3 }),
      reference,
    )
    expect(derived.iflc_a).toBeCloseTo(1.7391304347826086, 12)
    expect(derived.iz_a).toBeCloseTo(17.16, 12)
    expect(derived.runs_required).toBeCloseTo(0.10134792743488395, 12)
    expect(derived.vd_v).toBeCloseTo(1.4866201602716835, 9)
    expect(derived.vd_pct).toBeCloseTo(0.646356591422471, 9)
    expect(derived.max_length_m!).toBeCloseTo(232.07003996027498, 6)
    expect(derived.checks.ampacity).toBe(true)
    expect(derived.checks.protection).toBe(true)
  })

  it('uses the root-three factors for a three-phase circuit', () => {
    const derived = computeCableDerived(
      circuit({
        load_kw: 25,
        length_m: 25,
        voltage_v: 400,
        phases: 'three',
        power_factor: 1,
        cores: 'four_core_earth',
        size_mm2: 35,
        grouping_factor: 0.8,
        allowable_vd_pct: 1,
      }),
      reference,
    )
    expect(derived.iflc_a).toBeCloseTo(36.08545034642032, 9)
    expect(derived.iz_a).toBeCloseTo(109.6, 9)
    expect(derived.vd_v).toBeCloseTo(1.0453125, 9)
    expect(derived.vd_pct).toBeCloseTo(0.261328125, 9)
  })

  it('flags an overloaded circuit', () => {
    const derived = computeCableDerived(
      circuit({ load_kw: 20, protection_rating_a: 100, allowable_vd_pct: 3 }),
      reference,
    )
    expect(derived.checks.ampacity).toBe(false)
    expect(derived.checks.protection).toBe(false)
    expect(derived.warnings.map((warning) => warning.code)).toContain('ampacity_exceeded')
  })

  it('flags a size that is not in the reference table', () => {
    const derived = computeCableDerived(circuit({ size_mm2: 400 }), reference)
    expect(derived.iz_a).toBe(0)
    expect(derived.warnings.map((warning) => warning.code)).toContain('cable_not_in_table')
  })

  it('labels cables the way the workbook does', () => {
    expect(cableLabel('two_core_earth', 2.5)).toBe('2C+E x 2.5 mm2')
    expect(cableLabel('four_core_earth', 35)).toBe('4C+E x 35 mm2')
  })
})
