import { describe, expect, it } from 'vitest'
import { blankCathodicItem, computeCathodicDerived } from './cathodic'

describe('cathodic protection mirror', () => {
  it('reproduces the vertical tank sheet (hk, raw water)', () => {
    const item = {
      ...blankCathodicItem(),
      structure: 'tank' as const,
      orientation: 'vertical' as const,
      diameter_m: 16.8,
      length_m: 17.2,
      liquid_height_m: 17.2,
      design_life_years: 20,
      coating_breakdown: 0.05,
      current_density_a_m2: 0.07,
      design_temp_c: 27,
      operating_temp_c: 27,
      capacity_ah_kg: 780,
      utilization: 0.85,
      discrepancy_pct: 10,
      anode_weight_kg: 40,
      anodes_selected: 40,
    }
    const derived = computeCathodicDerived(item)
    expect(derived.current_density_a_m2).toBeCloseTo(0.0036, 12)
    expect(derived.capacity_at_temp_ah_kg).toBeCloseTo(591, 9)
    expect(derived.wetted_area_m2).toBeCloseTo(1129.4653908186024, 6)
    expect(derived.exposed_area_m2).toBeCloseTo(1242.4153908186024, 6)
    expect(derived.protection_current_a).toBeCloseTo(4.472695406946969, 9)
    expect(derived.anodes_required).toBe(39)
    expect(derived.acceptable).toBe(true)
  })

  it('reproduces the horizontal vessel sheet (mat381-D-1002)', () => {
    const item = {
      ...blankCathodicItem(),
      structure: 'tank' as const,
      orientation: 'horizontal' as const,
      diameter_m: 1.2,
      length_m: 2.6,
      liquid_height_m: 0.4,
      design_life_years: 20,
      coating_breakdown: 0.1,
      apply_coating_breakdown: false,
      current_density_a_m2: 0.07,
      design_temp_c: 55,
      operating_temp_c: 55,
      capacity_ah_kg: 2500,
      utilization: 0.85,
      discrepancy_pct: 20,
      anode_weight_kg: 10.5,
      anodes_selected: 7,
    }
    const derived = computeCathodicDerived(item)
    expect(derived.current_density_a_m2).toBeCloseTo(0.1, 12)
    expect(derived.exposed_area_m2).toBeCloseTo(5.278229715025711, 6)
    expect(derived.protection_current_a).toBeCloseTo(0.5278229715025712, 9)
    expect(derived.anode_mass_kg).toBeCloseTo(69.96374852071155, 6)
    expect(derived.anodes_required).toBe(7)
  })

  it('fails the anode count when the ICCP groundbed output is zero', () => {
    const item = {
      ...blankCathodicItem(),
      method: 'iccp' as const,
      structure: 'pipeline' as const,
      diameter_m: 0.5,
      length_m: 100,
      current_density_a_m2: 0.05,
      apply_coating_breakdown: false,
      discrepancy_pct: 0,
      anodes_selected: 1,
      rectifier: {
        anode_output_a: 0,
        groundbed_resistance_ohm: 1,
        cable_resistance_ohm: 0.2,
        back_emf_v: 2,
        margin_pct: 20,
      },
    }
    const derived = computeCathodicDerived(item)
    expect(derived.protection_current_a).toBeGreaterThan(0)
    expect(derived.checks.anode_count).toBe(false)
    expect(derived.acceptable).toBe(false)
  })

  it('uses the wall height for an external rectangular tank', () => {
    const item = {
      ...blankCathodicItem(),
      structure: 'tank' as const,
      orientation: 'rectangular' as const,
      surface: 'external' as const,
      length_m: 10,
      width_m: 6,
      height_m: 3,
      liquid_height_m: 1,
      discrepancy_pct: 0,
    }
    const derived = computeCathodicDerived(item)
    expect(derived.exposed_area_m2).toBeCloseTo(216, 9)
  })

  it('inherits the operating temperature when the capacity temperature is blank', () => {
    const base = {
      ...blankCathodicItem(),
      capacity_ah_kg: 2500,
      capacity_temp_c: null,
      operating_temp_c: 65,
    }
    expect(computeCathodicDerived(base).capacity_at_temp_ah_kg).toBeCloseTo(1285, 9)
    expect(
      computeCathodicDerived({ ...base, capacity_temp_c: 25 }).capacity_at_temp_ah_kg,
    ).toBeCloseTo(2365, 9)
  })
})
