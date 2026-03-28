export interface VendorBrand {
  name: string;
  slug: string;
  monogram: string;
  bgClass: string;
  ringClass: string;
}

const BRAND_PRESETS: Record<string, Omit<VendorBrand, 'name'>> = {
  Apple: { slug: 'apple', monogram: 'A', bgClass: 'from-slate-700 to-slate-900 text-white', ringClass: 'ring-slate-400/40' },
  ASUS: { slug: 'asus', monogram: 'AS', bgClass: 'from-blue-600 to-cyan-500 text-white', ringClass: 'ring-blue-400/40' },
  Aruba: { slug: 'aruba', monogram: 'AR', bgClass: 'from-orange-500 to-amber-400 text-black', ringClass: 'ring-orange-300/50' },
  Broadcom: { slug: 'broadcom', monogram: 'BC', bgClass: 'from-red-600 to-rose-500 text-white', ringClass: 'ring-rose-300/40' },
  Cisco: { slug: 'cisco', monogram: 'CI', bgClass: 'from-sky-600 to-blue-500 text-white', ringClass: 'ring-sky-300/40' },
  Dell: { slug: 'dell', monogram: 'DE', bgClass: 'from-blue-700 to-blue-500 text-white', ringClass: 'ring-blue-300/40' },
  'D-Link': { slug: 'dlink', monogram: 'DL', bgClass: 'from-emerald-500 to-teal-500 text-white', ringClass: 'ring-emerald-300/40' },
  Google: { slug: 'google', monogram: 'G', bgClass: 'from-yellow-400 via-red-500 to-blue-600 text-white', ringClass: 'ring-blue-300/40' },
  H3C: { slug: 'h3c', monogram: 'H3', bgClass: 'from-red-700 to-orange-500 text-white', ringClass: 'ring-orange-300/40' },
  Huawei: { slug: 'huawei', monogram: 'HW', bgClass: 'from-red-600 to-red-400 text-white', ringClass: 'ring-red-300/40' },
  Intel: { slug: 'intel', monogram: 'IN', bgClass: 'from-sky-500 to-indigo-500 text-white', ringClass: 'ring-sky-300/40' },
  Linksys: { slug: 'linksys', monogram: 'LS', bgClass: 'from-blue-500 to-indigo-700 text-white', ringClass: 'ring-indigo-300/40' },
  'Locally Administered': { slug: 'local', monogram: 'LA', bgClass: 'from-slate-600 to-slate-500 text-white', ringClass: 'ring-slate-300/30' },
  Microsoft: { slug: 'microsoft', monogram: 'MS', bgClass: 'from-cyan-600 to-sky-500 text-white', ringClass: 'ring-cyan-300/40' },
  Netgear: { slug: 'netgear', monogram: 'NG', bgClass: 'from-teal-600 to-emerald-500 text-white', ringClass: 'ring-emerald-300/40' },
  Qualcomm: { slug: 'qualcomm', monogram: 'QC', bgClass: 'from-indigo-700 to-fuchsia-600 text-white', ringClass: 'ring-fuchsia-300/40' },
  Samsung: { slug: 'samsung', monogram: 'SS', bgClass: 'from-blue-700 to-sky-500 text-white', ringClass: 'ring-sky-300/40' },
  'TP-Link': { slug: 'tplink', monogram: 'TP', bgClass: 'from-emerald-600 to-lime-500 text-white', ringClass: 'ring-lime-300/40' },
  Ubiquiti: { slug: 'ubiquiti', monogram: 'UB', bgClass: 'from-cyan-500 to-teal-400 text-slate-950', ringClass: 'ring-cyan-300/40' },
  VMware: { slug: 'vmware', monogram: 'VM', bgClass: 'from-orange-500 to-amber-500 text-white', ringClass: 'ring-orange-300/40' },
  Xiaomi: { slug: 'xiaomi', monogram: 'MI', bgClass: 'from-orange-600 to-amber-500 text-white', ringClass: 'ring-amber-300/40' },
  ZTE: { slug: 'zte', monogram: 'ZT', bgClass: 'from-blue-500 to-cyan-500 text-white', ringClass: 'ring-cyan-300/40' }
};

function fallbackMonogram(name: string): string {
  return name
    .split(/[\s-]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? '')
    .join('') || '?';
}

export function getVendorBrand(name?: string | null): VendorBrand {
  if (!name || name === 'Unknown') {
    return {
      name: 'Unknown',
      slug: 'unknown',
      monogram: '?',
      bgClass: 'from-slate-500 to-slate-700 text-white',
      ringClass: 'ring-slate-300/30'
    };
  }

  const preset = BRAND_PRESETS[name];
  if (preset) {
    return { name, ...preset };
  }

  return {
    name,
    slug: name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/(^-|-$)/g, ''),
    monogram: fallbackMonogram(name),
    bgClass: 'from-violet-600 to-indigo-700 text-white',
    ringClass: 'ring-violet-300/30'
  };
}
