<#
  perf/measure.ps1 — instrumentación reproducible de whatsapp-tauri (Tauri v2 / WebView2).

  Mide, en una sola pasada:
    * tamaño del binario release y del bundle instalador
    * arranque: t_window (HWND visible) y t_ready (primer target CDP del webview)
    * RAM/CPU en idle tras warmup (muestreo cada 2 s, mediana y pico)
    * FPS y métricas de tarea del webview (delega en perf/cdp-probe.mjs)

  Uso:
    pwsh -NoProfile -File perf/measure.ps1 -Runs 5 -IdleSeconds 60 -Label baseline

  Reglas (§2.2 del runbook):
    - 3 tomas mínimo por KPI; se reporta la mediana.
    - Máquina en reposo: no ejecutar esto con builds o clippy en marcha.
    - No se borra ningún dato de usuario (no se toca %APPDATA% de la app).
#>
[CmdletBinding()]
param(
  [string]$Exe = 'D:\dev\whatsapp-tauri\whatsapp-lite\src-tauri\target\release\whatsapp-tauri.exe',
  [string]$RepoRoot = 'D:\dev\whatsapp-tauri',
  [string]$NodeExe = 'node',
  [int]$Port = 9222,
  [int]$Runs = 5,
  [int]$IdleSeconds = 60,
  [int]$WarmupSeconds = 25,
  [string]$Label = (Get-Date -Format 'yyyyMMdd-HHmmss'),
  [switch]$SkipStartup,
  [switch]$SkipIdle
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Write-Step([string]$msg) { Write-Host "[measure] $msg" -ForegroundColor Cyan }

function Get-Median([double[]]$values) {
  if (-not $values -or $values.Count -eq 0) { return $null }
  $sorted = @($values | Sort-Object)
  $n = $sorted.Count
  if ($n % 2 -eq 1) { return [double]$sorted[[int](($n - 1) / 2)] }
  return [double](($sorted[$n / 2 - 1] + $sorted[$n / 2]) / 2)
}

function Get-AppProcesses {
  $main = @(Get-Process -Name 'whatsapp-tauri' -ErrorAction SilentlyContinue)
  if ($main.Count -eq 0) { return @() }
  $all = @($main)
  $queue = New-Object System.Collections.Generic.Queue[int]
  foreach ($p in $main) { $queue.Enqueue($p.Id) }
  while ($queue.Count -gt 0) {
    $parentId = $queue.Dequeue()
    $children = Get-CimInstance Win32_Process -Filter "ParentProcessId=$parentId" -ErrorAction SilentlyContinue
    foreach ($c in $children) {
      $proc = Get-Process -Id $c.ProcessId -ErrorAction SilentlyContinue
      if ($proc) { $all += $proc; $queue.Enqueue($proc.Id) }
    }
  }
  return @($all | Sort-Object Id -Unique)
}

function Stop-App {
  foreach ($p in (Get-AppProcesses)) { try { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue } catch {} }
  Start-Sleep -Milliseconds 800
  foreach ($p in @(Get-Process -Name 'whatsapp-tauri' -ErrorAction SilentlyContinue)) {
    try { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue } catch {}
  }
  Start-Sleep -Milliseconds 500
}

function Get-ArtifactSizes {
  $result = @{}
  if (Test-Path $Exe) {
    $len = (Get-Item $Exe).Length
    $result.exe_bytes = $len
    $result.exe_mb = [math]::Round($len / 1MB, 2)
  }
  $bundle = Join-Path $RepoRoot 'whatsapp-lite\src-tauri\target\release\bundle'
  if (Test-Path $bundle) {
    $files = @(Get-ChildItem $bundle -Recurse -File)
    $sum = ($files | Measure-Object -Property Length -Sum).Sum
    $result.bundle_files = $files.Count
    $result.bundle_bytes = $sum
    $result.bundle_mb = [math]::Round($sum / 1MB, 2)
    $result.installers = @($files | ForEach-Object { @{ name = $_.Name; mb = [math]::Round($_.Length / 1MB, 2) } })
  }
  $icons = Join-Path $RepoRoot 'whatsapp-lite\src-tauri\icons'
  if (Test-Path $icons) {
    $iconFiles = @(Get-ChildItem $icons -Recurse -File)
    $result.icons_bytes = ($iconFiles | Measure-Object -Property Length -Sum).Sum
  }
  return $result
}

function Wait-CdpTarget([int]$timeoutMs = 30000) {
  $deadline = (Get-Date).AddMilliseconds($timeoutMs)
  while ((Get-Date) -lt $deadline) {
    try {
      $r = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/json/list" -TimeoutSec 1 -ErrorAction Stop
      $page = @($r | Where-Object { $_.type -eq 'page' })
      if ($page.Count -gt 0) { return $true }
    } catch {}
    Start-Sleep -Milliseconds 50
  }
  return $false
}

function Measure-ColdStart {
  param([int]$Index)
  Stop-App
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port"
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $proc = Start-Process -FilePath $Exe -PassThru
  $tWindow = $null
  $deadline = (Get-Date).AddSeconds(45)
  while ((Get-Date) -lt $deadline) {
    $p = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
    if ($p -and $p.MainWindowHandle -ne 0) { $tWindow = $sw.Elapsed.TotalMilliseconds; break }
    Start-Sleep -Milliseconds 20
  }
  $tCdp = $null
  if (Wait-CdpTarget -timeoutMs 45000) { $tCdp = $sw.Elapsed.TotalMilliseconds }
  $sw.Stop()
  # El paint (FCP) se lee una sola vez en la fase de idle: el target CDP persiste y
  # performance.getEntriesByType('paint') sigue siendo el de esta navegación.
  return [pscustomobject]@{
    run            = $Index
    t_window_ms    = if ($tWindow) { [math]::Round($tWindow, 1) } else { $null }
    t_cdp_ready_ms = if ($tCdp) { [math]::Round($tCdp, 1) } else { $null }
    pid            = $proc.Id
  }
}

function Measure-Idle {
  param([int]$Warmup = 25, [int]$Duration = 60)
  Start-Sleep -Seconds $Warmup
  $procs = Get-AppProcesses
  if ($procs.Count -eq 0) { return $null }
  $cpu0 = ($procs | Measure-Object -Property CPU -Sum).Sum
  $samples = @()
  $elapsed = 0
  while ($elapsed -lt $Duration) {
    Start-Sleep -Seconds 2
    $elapsed += 2
    $p2 = Get-AppProcesses
    $ws = ($p2 | Measure-Object -Property WorkingSet64 -Sum).Sum
    $priv = ($p2 | Measure-Object -Property PrivateMemorySize64 -Sum).Sum
    $cpu = ($p2 | Measure-Object -Property CPU -Sum).Sum
    # Desglose: el proceso nativo (Rust) frente al árbol WebView2, que es de Microsoft
    # y depende de web.whatsapp.com. Sin este corte, el KPI de RAM no es atribuible.
    $mainWs = ($p2 | Where-Object { $_.ProcessName -eq 'whatsapp-tauri' } | Measure-Object -Property WorkingSet64 -Sum).Sum
    $webWs = ($p2 | Where-Object { $_.ProcessName -like 'msedgewebview2*' } | Measure-Object -Property WorkingSet64 -Sum).Sum
    $samples += [pscustomobject]@{
      t            = $elapsed
      rss_mb       = [math]::Round($ws / 1MB, 1)
      rss_main_mb  = [math]::Round($mainWs / 1MB, 1)
      rss_web_mb   = [math]::Round($webWs / 1MB, 1)
      private_mb   = [math]::Round($priv / 1MB, 1)
      cpu_pct      = [math]::Round((($cpu - $cpu0) / $elapsed) * 100 / [Environment]::ProcessorCount, 2)
      handles      = ($p2 | ForEach-Object { $_.HandleCount } | Measure-Object -Sum).Sum
      procs        = $p2.Count
    }
  }
  $rssVals = @($samples | ForEach-Object { $_.rss_mb })
  $cpuVals = @($samples | ForEach-Object { $_.cpu_pct })
  return [pscustomobject]@{
    duration_s     = $Duration
    rss_median_mb  = Get-Median $rssVals
    rss_main_median_mb = Get-Median @($samples | ForEach-Object { $_.rss_main_mb })
    rss_web_median_mb  = Get-Median @($samples | ForEach-Object { $_.rss_web_mb })
    procs_median       = Get-Median @($samples | ForEach-Object { [double]$_.procs })
    rss_max_mb     = ($rssVals | Measure-Object -Maximum).Maximum
    rss_first_mb   = $rssVals[0]
    cpu_median_pct = Get-Median $cpuVals
    cpu_max_pct    = ($cpuVals | Measure-Object -Maximum).Maximum
    cores          = [Environment]::ProcessorCount
    samples        = $samples
  }
}

$outDir = Join-Path $RepoRoot 'perf\out'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

Write-Step "app=$Exe"
Write-Step ("label=$Label runs=$Runs idle=" + $IdleSeconds + "s")
$report = [ordered]@{
  label       = $Label
  started_utc = (Get-Date).ToUniversalTime().ToString('o')
  exe         = $Exe
  artifacts   = Get-ArtifactSizes
  startup     = @()
  idle        = $null
  webview     = $null
}

if (-not $SkipStartup) {
  $runsList = @()
  for ($i = 1; $i -le $Runs; $i++) {
    Write-Step "arranque $i/$Runs"
    $r = Measure-ColdStart -Index $i
    $runsList += $r
    Write-Host ("  t_window={0} ms  t_cdp={1} ms" -f $r.t_window_ms, $r.t_cdp_ready_ms)
    Start-Sleep -Seconds 2
  }
  $report.startup = $runsList
  $validWindows = @($runsList | Where-Object { $_.t_window_ms } | ForEach-Object { [double]$_.t_window_ms })
  $validCdp = @($runsList | Where-Object { $_.t_cdp_ready_ms } | ForEach-Object { [double]$_.t_cdp_ready_ms })
  $report.startup_summary = [ordered]@{
    t_window_median_ms = Get-Median $validWindows
    t_cdp_median_ms    = Get-Median $validCdp
    runs_ok            = $runsList.Count
  }
}

if (-not $SkipIdle) {
  Stop-App
  Write-Step ("arrancando para idle (warmup " + $WarmupSeconds + "s + " + $IdleSeconds + "s)")
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port"
  $proc = Start-Process -FilePath $Exe -PassThru
  Start-Sleep -Seconds 3
  $null = Wait-CdpTarget -timeoutMs 45000
  $report.idle = Measure-Idle -Warmup $WarmupSeconds -Duration $IdleSeconds
  Write-Step "capturando FPS/métricas del webview (5 s)"
  try {
    $rawProbe = & $NodeExe (Join-Path $RepoRoot 'perf\cdp-probe.mjs') --port $Port --ms 5000 --deadline 30000 2>$null
    $report.webview = $rawProbe | ConvertFrom-Json
  } catch { $report.webview = @{ error = "$($_.Exception.Message)" } }
  Stop-App
}

$report.finished_utc = (Get-Date).ToUniversalTime().ToString('o')
$jsonPath = Join-Path $outDir "$Label.json"
$report | ConvertTo-Json -Depth 10 | Set-Content -Path $jsonPath -Encoding UTF8
Write-Step "resultado -> $jsonPath"
$report | ConvertTo-Json -Depth 4 -Compress
