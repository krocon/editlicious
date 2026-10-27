$ErrorActionPreference = 'Stop'

# Per-machine MSI from the GitHub release (Chocolatey runs elevated; the per-user NSIS installer would land in the
# admin's profile). sha256 from the release API, re-checked locally.
$packageArgs = @{
  packageName    = $env:ChocolateyPackageName
  fileType       = 'msi'
  url64bit       = 'https://github.com/krocon/editlicious/releases/download/v1.0.0/editlicious_1.0.0_x64_en-US.msi'
  checksum64     = '6ce8722d1895eeb90c0a43f06b97de2f0071b3eb301d418a46870a6b4d03941e'
  checksumType64 = 'sha256'
  softwareName   = 'editlicious*'
  silentArgs     = "/qn /norestart /l*v `"$($env:TEMP)\$($env:ChocolateyPackageName).$($env:ChocolateyPackageVersion).MsiInstall.log`""
  validExitCodes = @(0, 3010, 1641)
}

Install-ChocolateyPackage @packageArgs
