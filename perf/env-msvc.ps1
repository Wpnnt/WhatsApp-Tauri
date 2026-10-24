# perf/env-msvc.ps1 — dot-source: deja lista la cadena MSVC + Windows SDK reubicados en D:\dev.
# El .cargo/config.toml del proyecto solo fija el linker; ni LIB, ni INCLUDE, ni rc.exe existen
# en el PATH del harness. Sin esto, cargo clippy / cargo test / cargo build fallan con
# LNK1181 'OleAut32.lib' o 'RC.EXE not found'.

$MsvcRoot = 'D:\dev\BuildTools\VC\Tools\MSVC\14.44.35207'
$SdkRoot  = 'D:\dev\BuildTools\Windows Kits\10'
$SdkVer   = '10.0.26100.0'

$env:CARGO_HOME = 'D:\dev\cargo'
$env:RUSTUP_HOME = 'D:\dev\rustup'
$env:PATH = @(
  'D:\dev\cargo\bin',
  "$MsvcRoot\bin\Hostx64\x64",
  "$SdkRoot\bin\$SdkVer\x64",
  $env:PATH
) -join ';'
$env:LIB = @(
  "$MsvcRoot\lib\x64",
  "$SdkRoot\Lib\$SdkVer\um\x64",
  "$SdkRoot\Lib\$SdkVer\ucrt\x64"
) -join ';'
$env:INCLUDE = @(
  "$MsvcRoot\include",
  "$SdkRoot\Include\$SdkVer\ucrt",
  "$SdkRoot\Include\$SdkVer\um",
  "$SdkRoot\Include\$SdkVer\shared",
  "$SdkRoot\Include\$SdkVer\winrt"
) -join ';'
$env:RC = "$SdkRoot\bin\$SdkVer\x64\rc.exe"
