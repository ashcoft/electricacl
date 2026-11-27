import { describe, expect, it } from 'vitest'
import { blankItem, computeDerived } from '../lib/calc'
import type { LoadItem, MotorRef } from '../types'

const motorTable: MotorRef[] = [
  {
    rated_kw: 15,
    starting_ratio: 7.9,
    eff_full: 0.906,
    eff_75: 0.911,
    eff_50: 0.905,
    pf_full: 0.84,
    pf_75: 0.79,
    pf_50: 0.69,
  },
  {
    rated_kw: 0.55,
    starting_ratio: 6.3,
    eff_full: 0.771,
    eff_75: 0.754,
    eff_50: 0.713,
    pf_full: 0.81,
    pf_75: 0.75,
    pf_50: 0.65,
  },
]

function item(overrides: Partial<LoadItem>): LoadItem {
  return { ...blankItem(), ...overrides }
}

describe('computeDerived — mirrors the Rust engine', () => {
  it('uses feeder defaults and the workbook formulas', () => {
    // START-UP HEATER row of the source workbook: 1500 kW at 690 V.
    const derived = computeDerived(
      item({ equipment_class: 'feeder', absorbed_kw: 1500, rated_kw: 1500, voltage_kv: 0.69 }),
      motorTable,
    )
    expect(derived.efficiency).toBe(0.95)
    expect(derived.power_factor).toBe(0.9)
    expect(derived.load_factor).toBe(1)
    expect(derived.consumed_kw).toBeCloseTo(1578.9473684210527, 9)
    expect(derived.consumed_kvar).toBeCloseTo(764.7191129018724, 9)
    expect(derived.flc_a).toBeCloseTo(1468.0071333402623, 9)
    expect(derived.consumed_kva).toBeCloseTo(1754.386, 3)
  })

  it('interpolates the motor curve between 50 % and 75 % load', () => {
    // 11 kW absorbed on a 15 kW motor -> 73.3 % load factor.
    const derived = computeDerived(
      item({ equipment_class: 'motor', absorbed_kw: 11, rated_kw: 15, voltage_kv: 0.69 }),
      motorTable,
    )
    expect(derived.efficiency).toBeCloseTo(0.9106, 9)
    expect(derived.power_factor).toBeCloseTo(0.7833333333333333, 9)
    expect(derived.flc_a).toBeCloseTo(17.59624608987188, 9)
    expect(derived.starting_current_a).toBeCloseTo(139.01, 2)
    expect(derived.starting_kva).toBeCloseTo(166.16, 1)
  })

  it('extrapolates below 50 % using the absorbed-power branch', () => {
    // The workbook branches on absorbed power, not load factor.
    const derived = computeDerived(
      item({ equipment_class: 'motor', absorbed_kw: 0.3, rated_kw: 0.55, voltage_kv: 0.69 }),
      motorTable,
    )
    expect(derived.efficiency).toBeCloseTo(0.7055454545454545, 9)
    expect(derived.power_factor).toBeCloseTo(0.6681818181818182, 9)
  })

  it('falls back to feeder defaults when the rating is not in the table', () => {
    const derived = computeDerived(
      item({ equipment_class: 'motor', absorbed_kw: 3, rated_kw: 3, voltage_kv: 0.69 }),
      motorTable,
    )
    expect(derived.basis).toBe('fallback')
    expect(derived.efficiency).toBe(0.95)
    expect(derived.warnings.map((warning) => warning.code)).toContain('motor_not_in_table')
  })

  it('treats other users as unity and guards a zero voltage', () => {
    const derived = computeDerived(
      item({ equipment_class: 'other', absorbed_kw: 5, rated_kw: 5, voltage_kv: 0 }),
      motorTable,
    )
    expect(derived.efficiency).toBe(1)
    expect(derived.power_factor).toBe(1)
    expect(derived.flc_a).toBe(0)
    expect(derived.consumed_kvar).toBe(0)
  })

  it('flags a missing duty, missing scenario and an overloaded consumer', () => {
    const derived = computeDerived(
      item({ duty: null, scenarios: [], absorbed_kw: 20, rated_kw: 10 }),
      motorTable,
    )
    const codes = derived.warnings.map((warning) => warning.code)
    expect(codes).toEqual(
      expect.arrayContaining(['no_duty', 'no_scenario', 'load_factor_high']),
    )
  })

  it('flags a consumer with no power at all', () => {
    const derived = computeDerived(item({ absorbed_kw: 0, rated_kw: 0 }), motorTable)
    expect(derived.warnings.map((warning) => warning.code)).toContain('zero_power')
  })
})
