# SPDX-License-Identifier: MPL-2.0
# Run from the isolated spike directory, after the offline release build.
$ErrorActionPreference = 'Stop'
$binary = Join-Path $PSScriptRoot 'target/release/benchmark.exe'
$outputPath = Join-Path $PSScriptRoot 'results/windows-offscreen.json'
$startInfo = [System.Diagnostics.ProcessStartInfo]::new()
$startInfo.FileName = $binary
$startInfo.Arguments = '--output "' + $outputPath + '"'
$startInfo.WorkingDirectory = $PSScriptRoot
$startInfo.UseShellExecute = $false
$startInfo.CreateNoWindow = $true
$process = [System.Diagnostics.Process]::Start($startInfo)
$peak = 0L
while (-not $process.WaitForExit(250)) {
    $process.Refresh()
    $peak = [Math]::Max($peak, $process.PeakWorkingSet64)
}
if ($process.ExitCode -ne 0) { throw "Benchmark failed: $($process.ExitCode)" }
# OS process high-water mark sampled during execution; excludes the compiler and this wrapper.
# This is a whole-run RSS measure, not GPU memory, private bytes, or per-strategy memory.
$benchmarkResult = Get-Content -LiteralPath $outputPath -Raw -Encoding UTF8 | ConvertFrom-Json
$result = [ordered]@{
    adapter = $benchmarkResult.adapter
    schema = 1
    metric = 'Windows Process.PeakWorkingSet64 sampled every 250 ms'
    whole_run_peak_working_set_bytes = $peak
    scope = 'benchmark process, all six cases; not per-strategy or GPU memory'
}
$json = ($result | ConvertTo-Json -Compress) + "`n"
[System.IO.File]::WriteAllText((Join-Path $PSScriptRoot 'results/windows-memory.json'), $json, [System.Text.UTF8Encoding]::new($false))
