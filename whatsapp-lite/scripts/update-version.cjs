const fs = require('fs');
const path = require('path');

const newVersion = process.argv[2];

if (!newVersion) {
  console.error('❌ Error: Please provide a version number. Example: npm run version 0.2.0');
  process.exit(1);
}

if (!/^\d+\.\d+\.\d+$/.test(newVersion)) {
    console.error('❌ Error: Invalid version format. Use SemVer (e.g., 0.2.0)');
    process.exit(1);
}

const paths = {
  package: path.join(__dirname, '../package.json'),
  tauri: path.join(__dirname, '../src-tauri/tauri.conf.json'),
  cargo: path.join(__dirname, '../src-tauri/Cargo.toml')
};

console.log(`🚀 Updating project to version ${newVersion}...`);

// 1. Update package.json
try {
  const pkg = JSON.parse(fs.readFileSync(paths.package, 'utf8'));
  pkg.version = newVersion;
  fs.writeFileSync(paths.package, JSON.stringify(pkg, null, 2) + '\n');
  console.log('✅ Updated package.json');
} catch (e) { console.error('❌ Failed to update package.json:', e.message); }

// 2. Update tauri.conf.json
try {
  const tauri = JSON.parse(fs.readFileSync(paths.tauri, 'utf8'));
  tauri.version = newVersion;
  fs.writeFileSync(paths.tauri, JSON.stringify(tauri, null, 2) + '\n');
  console.log('✅ Updated tauri.conf.json');
} catch (e) { console.error('❌ Failed to update tauri.conf.json:', e.message); }

// 3. Update Cargo.toml
try {
  let cargo = fs.readFileSync(paths.cargo, 'utf8');
  cargo = cargo.replace(/^version = ".*?"/m, `version = "${newVersion}"`);
  fs.writeFileSync(paths.cargo, cargo);
  console.log('✅ Updated Cargo.toml');
} catch (e) { console.error('❌ Failed to update Cargo.toml:', e.message); }

console.log(`\n🎉 Success! All files are now on v${newVersion}`);
console.log(`👉 Next steps:`);
console.log(`   1. git add .`);
console.log(`   2. git commit -m "chore: bump version to ${newVersion}"`);
console.log(`   3. git tag -a v${newVersion} -m "Release v${newVersion}"`);
console.log(`   4. git push origin main --tags`);
