#!/usr/bin/env node
const sharp = require('sharp');
const fs = require('fs');
const path = require('path');

const inp = path.resolve(process.cwd(), 'src-tauri', 'icons', 'whatsapp-tauri.svg');
const outFile = path.resolve(process.cwd(), 'src-tauri', 'app-icon.png');

async function main() {
  if (!fs.existsSync(inp)) {
    console.error('SVG source not found:', inp);
    process.exit(1);
  }

  try {
    await sharp(inp, { density: 1200 })
      .resize(2048, 2048, {
        fit: 'contain',
        background: { r: 0, g: 0, b: 0, alpha: 0 }
      })
      .png({ compressionLevel: 9, adaptiveFiltering: true })
      .toFile(outFile);

    console.log('wrote', outFile);
  } catch (err) {
    console.error(err);
    process.exit(1);
  }
}

main();
