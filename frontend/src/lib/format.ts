const numberFormatCache = new Map<number, Intl.NumberFormat>()

function formatter(digits: number): Intl.NumberFormat {
  let cached = numberFormatCache.get(digits)
  if (!cached) {
    cached = new Intl.NumberFormat('en-US', {
      minimumFractionDigits: digits,
      maximumFractionDigits: digits,
    })
    numberFormatCache.set(digits, cached)
  }
  return cached
}

/** Format a number with thousands separators, `digits` decimals. */
export function fmt(value: number | null | undefined, digits = 1): string {
  if (value === null || value === undefined || Number.isNaN(value)) return '–'
  return formatter(digits).format(value)
}

/** Compact display for small and large values in dense tables. */
export function fmtSmart(value: number | null | undefined): string {
  if (value === null || value === undefined || Number.isNaN(value)) return '–'
  const abs = Math.abs(value)
  if (abs === 0) return '0'
  if (abs < 0.1) return formatter(3).format(value)
  if (abs < 10) return formatter(2).format(value)
  return formatter(1).format(value)
}

export function fmtPct(fraction: number | null | undefined, digits = 1): string {
  if (fraction === null || fraction === undefined || Number.isNaN(fraction)) return '–'
  return `${formatter(digits).format(fraction * 100)} %`
}

export function fmtVoltage(kv: number): string {
  return `${Number(kv.toFixed(3))} kV`
}

export function fmtDate(value: Date = new Date()): string {
  return value.toLocaleDateString('en-GB', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
  })
}
