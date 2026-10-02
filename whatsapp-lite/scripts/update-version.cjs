const fs = require('fs');
const path = require('path');

const newVersion = process.argv[2];

if (!newVersion) {
  console.error('❌ Error: Please provide a version number. Example: npm run version 0.2.0');
  process.exit(1);
}

if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(newVersion)) {
    console.error('❌ Error: Invalid version format. Use SemVer (e.g., 0.2.0)');
    process.exit(1);
}

const paths = {
  package: path.join(__dirname, '../package.json'),
  packageLock: path.join(__dirname, '../package-lock.json'),
  tauri: path.join(__dirname, '../src-tauri/tauri.conf.json'),
  cargo: path.join(__dirname, '../src-tauri/Cargo.toml')
};

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function updateCargoVersion(contents) {
  const packageStart = contents.search(/^\[package\][ \t]*$/m);
  if (packageStart === -1) throw new Error('Could not find [package] in Cargo.toml');

  const contentStart = contents.indexOf('\n', packageStart) + 1;
  const nextSection = contents.indexOf('\n[', contentStart);
  const sectionEnd = nextSection === -1 ? contents.length : nextSection + 1;
  const packageSection = contents.slice(packageStart, sectionEnd);
  if (!/^version\s*=\s*"[^"\r\n]+"\s*$/m.test(packageSection)) {
    throw new Error('Could not find [package].version in Cargo.toml');
  }
  return contents.slice(0, packageStart) + packageSection.replace(
    /^(version\s*=\s*)"[^"\r\n]+"\s*$/m,
    `$1"${newVersion}"`
  ) + contents.slice(sectionEnd);
}

try {
  const pkg = readJson(paths.package);
  const lock = readJson(paths.packageLock);
  const tauri = readJson(paths.tauri);
  const cargo = fs.readFileSync(paths.cargo, 'utf8');

  if (!lock.packages || !lock.packages['']) {
    throw new Error('package-lock.json is missing the root package entry');
  }
  if (lock.name !== pkg.name || lock.packages[''].name !== pkg.name) {
    throw new Error('package-lock.json root package name does not match package.json');
  }

  pkg.version = newVersion;
  lock.version = newVersion;
  lock.packages[''].version = newVersion;
  tauri.version = newVersion;
  const updatedCargo = updateCargoVersion(cargo);

  const updates = [
    [paths.package, `${JSON.stringify(pkg, null, 2)}\n`],
    [paths.packageLock, `${JSON.stringify(lock, null, 2)}\n`],
    [paths.tauri, `${JSON.stringify(tauri, null, 2)}\n`],
    [paths.cargo, updatedCargo]
  ];
  const staged = [];
  const transactionId = `${process.pid}-${Date.now()}-${Math.random().toString(36).slice(2)}`;

  try {
    for (const [filePath, contents] of updates) {
      const tempPath = `${filePath}.${transactionId}.tmp`;
      const backupPath = `${filePath}.${transactionId}.bak`;
      const mode = fs.statSync(filePath).mode;
      staged.push({ tempPath, backupPath, filePath });
      fs.writeFileSync(tempPath, contents, { mode });
    }
    for (const item of staged) {
      fs.renameSync(item.filePath, item.backupPath);
      fs.renameSync(item.tempPath, item.filePath);
    }
  } catch (error) {
    const rollbackErrors = [];
    for (const item of staged.slice().reverse()) {
      try {
        if (fs.existsSync(item.backupPath)) {
          if (fs.existsSync(item.filePath)) fs.unlinkSync(item.filePath);
          fs.renameSync(item.backupPath, item.filePath);
        }
        if (fs.existsSync(item.tempPath)) fs.unlinkSync(item.tempPath);
      } catch (rollbackError) {
        rollbackErrors.push(`${item.filePath}: ${rollbackError.message}`);
      }
    }
    if (rollbackErrors.length) {
      throw new Error(`${error.message}; rollback failed for ${rollbackErrors.join('; ')}`);
    }
    throw error;
  }

  for (const item of staged) {
    try {
      fs.unlinkSync(item.backupPath);
    } catch (error) {
      console.error(`Warning: could not remove backup ${item.backupPath}: ${error.message}`);
    }
  }

  console.log(`Updated project metadata to ${newVersion} in package.json, package-lock.json, tauri.conf.json, and Cargo.toml.`);
} catch (error) {
  console.error(`Failed to update project version: ${error.message}`);
  process.exitCode = 1;
}
