# Installs ghdeck on Windows from the latest GitHub release.
#   irm https://raw.githubusercontent.com/jitendravjh/ghdeck/main/install.ps1 | iex
# $env:GHDECK_VERSION picks a release like v0.2.2, $env:GHDECK_BIN_DIR picks where it goes.
& {
    $ErrorActionPreference = "Stop"
    $ProgressPreference = "SilentlyContinue"
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

    $repo = "jitendravjh/ghdeck"
    $version = if ($env:GHDECK_VERSION) { $env:GHDECK_VERSION } else { "latest" }
    $binDir = if ($env:GHDECK_BIN_DIR) { $env:GHDECK_BIN_DIR } else { Join-Path $env:LOCALAPPDATA "Programs\ghdeck" }
    $file = "ghdeck-x86_64-pc-windows-msvc.zip"

    if ($version -eq "latest") {
        $base = "https://github.com/$repo/releases/latest/download"
    } elseif ($version.StartsWith("v")) {
        $base = "https://github.com/$repo/releases/download/$version"
    } else {
        $base = "https://github.com/$repo/releases/download/v$version"
    }

    $tmp = Join-Path ([IO.Path]::GetTempPath()) ("ghdeck-" + [Guid]::NewGuid())
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        Write-Host "downloading $file"
        Invoke-WebRequest "$base/$file" -OutFile (Join-Path $tmp $file) -UseBasicParsing
        Invoke-WebRequest "$base/$file.sha256" -OutFile (Join-Path $tmp "$file.sha256") -UseBasicParsing

        $want = ((Get-Content (Join-Path $tmp "$file.sha256") -Raw).Trim() -split '\s+')[0].ToLower()
        $got = (Get-FileHash (Join-Path $tmp $file) -Algorithm SHA256).Hash.ToLower()
        if ($got -ne $want) { throw "checksum mismatch for $file, not installing it" }

        Expand-Archive (Join-Path $tmp $file) -DestinationPath $tmp -Force
        New-Item -ItemType Directory -Path $binDir -Force | Out-Null
        Copy-Item (Join-Path $tmp "ghdeck.exe") (Join-Path $binDir "ghdeck.exe") -Force
    } finally {
        Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }

    $exe = Join-Path $binDir "ghdeck.exe"
    Write-Host "installed $(& $exe --version) to $exe"

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (-not (($userPath -split ";") -contains $binDir)) {
        $newPath = if ($userPath) { "$userPath;$binDir" } else { $binDir }
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Host "added $binDir to your PATH, open a new terminal to use ghdeck"
    }
    Write-Host "next, connect your account: https://ghdeck.jitendravjh.in/docs/#connect-your-account"
}
