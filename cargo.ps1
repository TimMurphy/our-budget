# Runs cargo with an MSVC toolset that has the C++ headers and libraries.
# Usage: ./cargo.ps1 run | ./cargo.ps1 test --test specs | ./cargo.ps1 build --release
# Needed on Windows when the shell's own Visual Studio has no C++ workload installed.

$roots = @("${env:ProgramFiles(x86)}\Microsoft Visual Studio", "$env:ProgramFiles\Microsoft Visual Studio")
$tool = Get-ChildItem $roots -Directory -Recurse -Depth 5 -Filter MSVC -ErrorAction SilentlyContinue |
    ForEach-Object { Get-ChildItem $_.FullName -Directory } |
    Where-Object { (Test-Path "$($_.FullName)\lib\x64\msvcrt.lib") -and (Test-Path "$($_.FullName)\include\stdarg.h") } |
    Sort-Object Name -Descending | Select-Object -First 1
if (-not $tool) { throw "No MSVC toolset with C++ headers found. Install the 'Desktop development with C++' workload." }

$kits = "${env:ProgramFiles(x86)}\Windows Kits\10"
$sdk = Get-ChildItem "$kits\Lib" -Directory | Sort-Object Name -Descending | Select-Object -First 1
$v = $sdk.Name
$m = $tool.FullName
$env:LIB = "$m\lib\x64;$kits\Lib\$v\ucrt\x64;$kits\Lib\$v\um\x64"
$env:INCLUDE = "$m\include;$kits\Include\$v\ucrt;$kits\Include\$v\um;$kits\Include\$v\shared"
$env:Path = "$m\bin\Hostx64\x64;$kits\bin\$v\x64;$env:Path;$env:USERPROFILE\.cargo\bin"

Set-Location $PSScriptRoot
& cargo @args
exit $LASTEXITCODE
