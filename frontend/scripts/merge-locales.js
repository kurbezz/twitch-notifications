#!/usr/bin/env node
import fs from 'fs/promises';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const ruPath = path.join(__dirname, '..', 'src', 'locales_ru.json');
const enPath = path.join(__dirname, '..', 'src', 'locales_en.json');
const outPath = path.join(__dirname, '..', 'src', 'locales_en.merged.json');

function mergeRecursive(en, ru) {
  // if en is undefined, copy ru
  if (en === undefined) return ru;
  if (typeof en === 'string' || typeof ru === 'string') {
    return en; // prefer existing en
  }
  const out = { ...en };
  for (const key of Object.keys(ru)) {
    if (!(key in out)) {
      out[key] = ru[key];
    } else {
      out[key] = mergeRecursive(out[key], ru[key]);
    }
  }
  return out;
}

async function main() {
  try {
    const [ruRaw, enRaw] = await Promise.all([
      fs.readFile(ruPath, 'utf8'),
      fs.readFile(enPath, 'utf8'),
    ]);
    const ru = JSON.parse(ruRaw);
    const en = JSON.parse(enRaw);
    const merged = mergeRecursive(en, ru);
    const out = JSON.stringify(merged, null, 2) + '\n';
    await fs.writeFile(outPath, out, 'utf8');
    console.log('Merged locales written to', outPath);
  } catch (err) {
    console.error('Error merging locales:', err);
    process.exit(1);
  }
}

main();
