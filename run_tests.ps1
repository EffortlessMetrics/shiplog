$ErrorActionPreference = "Continue"

cd 'H:\Code\Rust\shiplog'

Write-Host "=== Test 1: shiplog-output-layout ===" -ForegroundColor Green
cargo test -p shiplog-output-layout 2>&1
$test1Exit = $LASTEXITCODE
Write-Host "Test 1 exit code: $test1Exit" -ForegroundColor Yellow

Write-Host "`n=== Test 2: shiplog-validate ===" -ForegroundColor Green
cargo test -p shiplog-validate 2>&1
$test2Exit = $LASTEXITCODE
Write-Host "Test 2 exit code: $test2Exit" -ForegroundColor Yellow

Write-Host "`n=== Test 3: shiplog-export ===" -ForegroundColor Green
cargo test -p shiplog-export 2>&1
$test3Exit = $LASTEXITCODE
Write-Host "Test 3 exit code: $test3Exit" -ForegroundColor Yellow

Write-Host "`n=== Summary ===" -ForegroundColor Green
Write-Host "Test 1 (shiplog-output-layout): $test1Exit"
Write-Host "Test 2 (shiplog-validate): $test2Exit"
Write-Host "Test 3 (shiplog-export): $test3Exit"
