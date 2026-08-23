[CmdletBinding()]
param(
    [ValidateSet('Vulkan', 'CUDA13.3', 'All')]
    [string]$Runtime = 'Vulkan',
    [string]$Destination = (Join-Path $PSScriptRoot '..\src-tauri\binaries')
)

$ErrorActionPreference = 'Stop'
$destinationRoot = [System.IO.Path]::GetFullPath($Destination)
New-Item -ItemType Directory -Path $destinationRoot -Force | Out-Null

function Install-PinnedRuntime {
    param(
        [Parameter(Mandatory)] [string]$Tag,
        [Parameter(Mandatory)] [string]$Folder,
        [Parameter(Mandatory)] [scriptblock]$AssetFilter
    )

    $headers = @{ 'User-Agent' = 'Locastra-runtime-setup' }
    $release = Invoke-RestMethod -Headers $headers -Uri "https://api.github.com/repos/ggml-org/llama.cpp/releases/tags/$Tag"
    $assets = @($release.assets | Where-Object $AssetFilter)
    if ($assets.Count -eq 0) {
        throw "No matching runtime assets were found in llama.cpp release $Tag."
    }

    $target = Join-Path $destinationRoot $Folder
    New-Item -ItemType Directory -Path $target -Force | Out-Null
    foreach ($asset in $assets) {
        $archive = Join-Path ([System.IO.Path]::GetTempPath()) ("locastra-" + [guid]::NewGuid().ToString('N') + '.zip')
        try {
            Write-Host "Downloading $($asset.name)..."
            Invoke-WebRequest -Headers $headers -Uri $asset.browser_download_url -OutFile $archive
            if ($asset.digest -and $asset.digest.StartsWith('sha256:')) {
                $expected = $asset.digest.Substring(7)
                $actual = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
                if (-not $actual.Equals($expected, [System.StringComparison]::OrdinalIgnoreCase)) {
                    throw "SHA-256 mismatch for $($asset.name)."
                }
            }
            Expand-Archive -LiteralPath $archive -DestinationPath $target -Force
        }
        finally {
            if (Test-Path -LiteralPath $archive) {
                Remove-Item -LiteralPath $archive -Force
            }
        }
    }

    if (-not (Test-Path -LiteralPath (Join-Path $target 'llama-server.exe') -PathType Leaf)) {
        throw "Runtime $Folder was extracted but llama-server.exe is missing."
    }
    Write-Host "Installed $Folder to $target"
}

if ($Runtime -in @('Vulkan', 'All')) {
    Install-PinnedRuntime -Tag 'b10333' -Folder 'llama-b10333-vulkan' -AssetFilter {
        $_.name -like '*bin-win-vulkan-x64.zip'
    }
}

if ($Runtime -in @('CUDA13.3', 'All')) {
    Install-PinnedRuntime -Tag 'b10357' -Folder 'llama-b10357-cuda13.3' -AssetFilter {
        ($_.name -like 'llama-*-cuda-13.3-x64.zip') -or ($_.name -like 'cudart-llama-*-cuda-13.3-x64.zip')
    }
}
