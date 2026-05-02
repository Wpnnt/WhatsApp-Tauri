#!/usr/bin/env node
const fs = require('fs');
const path = require('path');

const root = process.cwd();
const iconsDir = path.join(root, 'src-tauri', 'icons');

const removeTargets = [
  path.join(iconsDir, 'android'),
  path.join(iconsDir, 'ios'),
  path.join(iconsDir, 'icon.icns'),
  path.join(iconsDir, 'StoreLogo.png'),
  path.join(iconsDir, 'Square30x30Logo.png'),
  path.join(iconsDir, 'Square44x44Logo.png'),
  path.join(iconsDir, 'Square71x71Logo.png'),
  path.join(iconsDir, 'Square89x89Logo.png'),
  path.join(iconsDir, 'Square107x107Logo.png'),
  path.join(iconsDir, 'Square142x142Logo.png'),
  path.join(iconsDir, 'Square150x150Logo.png'),
  path.join(iconsDir, 'Square284x284Logo.png'),
  path.join(iconsDir, 'Square310x310Logo.png'),
  path.join(iconsDir, '16x16.png'),
  path.join(iconsDir, '24x24.png'),
  path.join(iconsDir, '32x32.png'),
  path.join(iconsDir, '48x48.png'),
  path.join(iconsDir, '64x64.png'),
  path.join(iconsDir, '128x128.png'),
  path.join(iconsDir, '128x128@2x.png'),
  path.join(iconsDir, '256x256.png'),
  path.join(iconsDir, '512x512.png')
];

for (const target of removeTargets) {
  if (fs.existsSync(target)) {
    fs.rmSync(target, { recursive: true, force: true });
    console.log('removed', target);
  }
}
