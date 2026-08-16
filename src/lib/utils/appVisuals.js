const electronSvg = `data:image/svg+xml;utf8,${encodeURIComponent(
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" fill="none">
    <rect x="4" y="4" width="40" height="40" rx="12" fill="#141A22" stroke="#252E3A" stroke-width="1.5" />
    <path d="M16 24c0-6 3.6-10 8-10s8 4 8 10-3.6 10-8 10-8-4-8-10Z" stroke="#7AA2F7" stroke-width="2.2" />
    <path d="M12 18c5.2 2.8 18.8 2.8 24 0" stroke="#7AA2F7" stroke-width="2.2" stroke-linecap="round" />
    <path d="M12 30c5.2-2.8 18.8-2.8 24 0" stroke="#7AA2F7" stroke-width="2.2" stroke-linecap="round" />
    <circle cx="24" cy="24" r="3.2" fill="#7AA2F7" />
  </svg>`
)}`;

function encodeSvg(svg) {
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

const discoverSvg = encodeSvg(
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" fill="none">
    <rect x="4" y="4" width="40" height="40" rx="12" fill="#141A22" stroke="#252E3A" stroke-width="1.5" />
    <circle cx="24" cy="24" r="10" stroke="#7AA2F7" stroke-width="2.2" />
    <path d="M24 16l3.4 6.8L34 26l-6.8 3.4L24 36l-3.4-6.6L14 26l6.6-3.2L24 16Z" fill="#38BDF8" />
  </svg>`
);

const mailSvg = encodeSvg(
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" fill="none">
    <rect x="4" y="4" width="40" height="40" rx="12" fill="#141A22" stroke="#252E3A" stroke-width="1.5" />
    <rect x="10" y="14" width="28" height="20" rx="4" fill="#1E293B" stroke="#7AA2F7" stroke-width="2" />
    <path d="M12 17l12 8.5 12-8.5" stroke="#7AA2F7" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
  </svg>`
);

const authSvg = encodeSvg(
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" fill="none">
    <rect x="4" y="4" width="40" height="40" rx="12" fill="#141A22" stroke="#252E3A" stroke-width="1.5" />
    <path d="M24 12c5.5 3.5 9.5 4.3 9.5 4.3v7.8c0 5.8-4 10-9.5 10.6-5.5-.6-9.5-4.8-9.5-10.6v-7.8s4-.8 9.5-4.3Z" fill="#1E293B" stroke="#7AA2F7" stroke-width="2" stroke-linejoin="round" />
    <circle cx="24" cy="22" r="2.5" fill="#7AA2F7" />
    <path d="M24 24.5v4.5" stroke="#7AA2F7" stroke-width="2" stroke-linecap="round" />
  </svg>`
);

const fallbackIconMap = new Map([
  ['work review', '/icons/256x256.png'],
  ['work-review', '/icons/256x256.png'],
  ['work_review', '/icons/256x256.png'],
  ['electron', electronSvg],
  ['electron helper', electronSvg],
  ['discover', discoverSvg],
  ['mail', mailSvg],
  ['邮件', mailSvg],
  ['system authentication', authSvg],
  ['coreautha', authSvg],
]);

function normalizeName(appName) {
  return typeof appName === 'string' ? appName.trim().toLowerCase() : '';
}

function buildMonogramIcon(appName) {
  const normalized = normalizeName(appName);
  if (!normalized) return null;

  const words = normalized
    .replace(/[^a-z0-9\u4e00-\u9fa5]+/gi, ' ')
    .trim()
    .split(/\s+/)
    .filter(Boolean);

  let label = '';
  if (words.length >= 2) {
    label = `${words[0][0]}${words[1][0]}`;
  } else if (words.length === 1) {
    label = words[0].slice(0, words[0].match(/[\u4e00-\u9fa5]/) ? 1 : 2);
  }

  label = label.toUpperCase();
  if (!label) return null;

  let hash = 0;
  for (let i = 0; i < normalized.length; i += 1) {
    hash = normalized.charCodeAt(i) + ((hash << 5) - hash);
  }

  const hue = Math.abs(hash) % 360;
  const bg = `hsl(${hue} 70% 94%)`;
  const fg = `hsl(${hue} 55% 32%)`;
  const fontSize = label.length > 1 ? 19 : 22;

  return `data:image/svg+xml;utf8,${encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48">
      <rect x="4" y="4" width="40" height="40" rx="12" fill="${bg}" />
      <text x="24" y="29" text-anchor="middle" font-family="Segoe UI, Arial, sans-serif" font-size="${fontSize}" font-weight="700" fill="${fg}">${label}</text>
    </svg>`
  )}`;
}

export function getFallbackAppIcon(appName) {
  const normalized = normalizeName(appName);
  if (!normalized) return null;

  if (fallbackIconMap.has(normalized)) {
    return fallbackIconMap.get(normalized);
  }

  if (normalized.includes('work review') || normalized.includes('work-review') || normalized.includes('work_review')) {
    return '/icons/256x256.png';
  }

  if (normalized.includes('electron')) {
    return electronSvg;
  }

  return buildMonogramIcon(appName);
}

export function resolveAppIconSrc(appName, base64) {
  if (typeof base64 === 'string' && base64.length > 100) {
    return `data:image/png;base64,${base64}`;
  }

  return getFallbackAppIcon(appName);
}
