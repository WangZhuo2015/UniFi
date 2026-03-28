$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$repoRoot = Split-Path -Parent $PSScriptRoot
$dataDir = Join-Path $repoRoot 'src-tauri\data'
$tmpOui = Join-Path $repoRoot 'tmp-oui.csv'
$tmpCid = Join-Path $repoRoot 'tmp-cid.csv'
$tmpOui36 = Join-Path $repoRoot 'tmp-oui36.csv'
$tmpMam = Join-Path $repoRoot 'tmp-mam.csv'
$outFile = Join-Path $dataDir 'vendor-prefixes.tsv'

if (!(Test-Path $dataDir)) {
    New-Item -ItemType Directory -Path $dataDir | Out-Null
}

function Get-RegistryRows {
    param(
        [string]$Uri,
        [string]$OutFile,
        [int[]]$AllowedLengths
    )

    try {
        Invoke-WebRequest -Uri $Uri -OutFile $OutFile
    } catch {
        Write-Warning "Skipping $Uri: $($_.Exception.Message)"
        return @()
    }

    Import-Csv $OutFile | ForEach-Object {
        $prefix = $_.Assignment.Trim().ToUpper()
        $name = $_.'Organization Name'.Trim()

        if ($AllowedLengths -contains $prefix.Length -and $name) {
            [pscustomobject]@{
                Prefix = $prefix
                Name = $name
            }
        }
    }
}

$oui = Get-RegistryRows -Uri 'https://standards-oui.ieee.org/oui/oui.csv' -OutFile $tmpOui -AllowedLengths @(6)
$cid = Get-RegistryRows -Uri 'https://standards-oui.ieee.org/cid/cid.csv' -OutFile $tmpCid -AllowedLengths @(6)
$oui36 = Get-RegistryRows -Uri 'https://standards-oui.ieee.org/oui36/oui36.csv' -OutFile $tmpOui36 -AllowedLengths @(9)
$mam = Get-RegistryRows -Uri 'https://standards-oui.ieee.org/mam/mam.csv' -OutFile $tmpMam -AllowedLengths @(7)

$merged = @($oui + $cid + $oui36 + $mam) |
    Where-Object { $_.Prefix -match '^[0-9A-F]+$' -and $_.Name } |
    Sort-Object Prefix -Unique

$lines = $merged | ForEach-Object {
    "{0}`t{1}" -f $_.Prefix, (($_.Name -replace "`t", ' ') -replace '\s+', ' ').Trim()
}

Set-Content -Path $outFile -Value $lines -Encoding utf8

Write-Host "Wrote $($merged.Count) vendor prefixes to $outFile"
