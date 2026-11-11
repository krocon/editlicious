$ErrorActionPreference = 'Stop'

# Per-machine MSI from the GitHub release (Chocolatey runs elevated; the per-user NSIS installer would land in the
# admin's profile). sha256 from the release API, re-checked locally.
$packageArgs = @{
  packageName    = $env:ChocolateyPackageName
  fileType       = 'msi'
  url64bit       = 'https://github.com/krocon/editlicious/releases/download/v1.2.0/editlicious_1.2.0_x64_en-US.msi'
  checksum64     = 'dc3ea0fb6a800c22e6e6d1960c2e9e0d3793e4a3d1cf40435f581f27b6e34982'
  checksumType64 = 'sha256'
  softwareName   = 'editlicious*'
  silentArgs     = "/qn /norestart /l*v `"$($env:TEMP)\$($env:ChocolateyPackageName).$($env:ChocolateyPackageVersion).MsiInstall.log`""
  validExitCodes = @(0, 3010, 1641)
}

Install-ChocolateyPackage @packageArgs
