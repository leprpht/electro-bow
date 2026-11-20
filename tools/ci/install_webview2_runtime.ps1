$ErrorActionPreference = 'Stop'

# The NuGet SDK supplies headers and the loader, not the browser runtime.
# Detect Evergreen using the registry locations documented by Microsoft.
$client = '{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
$registryPaths = @(
    "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\$client",
    "HKCU:\Software\Microsoft\EdgeUpdate\Clients\$client"
)

function Get-WebView2Version {
    foreach ($path in $registryPaths) {
        $version = (Get-ItemProperty -Path $path -Name pv -ErrorAction SilentlyContinue).pv
        if ($version -and [version]$version -gt [version]'0.0.0.0') {
            return $version
        }
    }
}

if (-not (Get-WebView2Version)) {
    # Use the standalone x64 installer: hosted images may disable Edge Update,
    # which the small online bootstrapper needs to download the runtime.
    $installer = Join-Path $env:RUNNER_TEMP 'MicrosoftEdgeWebView2RuntimeInstallerX64.exe'
    Invoke-WebRequest -Uri 'https://go.microsoft.com/fwlink/?linkid=2124701' -OutFile $installer
    $process = Start-Process -FilePath $installer -ArgumentList '/silent', '/install' -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw "WebView2 Runtime installation failed with exit code $($process.ExitCode)"
    }
}

$version = Get-WebView2Version
if (-not $version) {
    throw 'WebView2 Runtime is unavailable after installation'
}
Write-Host "WebView2 Runtime: $version"
