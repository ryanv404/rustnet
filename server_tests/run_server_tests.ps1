# Initialize global variables
$crateDir = ($PSScriptRoot | Get-Item).Parent
$testsDir = Join-Path -Path $crateDir -ChildPath 'server_tests'
$outputFile = Join-Path -Path $testsDir -ChildPath 'test_output.txt'
$expectedFile = Join-Path -Path $testsDir -ChildPath 'expected_output.txt'

function Initialize-MyServer {
	$red = @{ ForegroundColor = 'Red'; }

	cargo clean *> $null

	if ($LASTEXITCODE -ne 0) {
		Write-Host @red "Unable to remove the prior build artifacts. Continuing."
	}

	Write-Host "Building..."

	cargo build --example server *> $null

	if ($LASTEXITCODE -ne 0) {
		Write-Host @red "Unable to build the server. Exiting."
		exit
	}
}

function Start-MyServer {
    if (Test-Path -Path $outputFile) {
        Remove-Item -Path $outputFile | Out-Null
	}

    New-Item -ItemType File -Path $outputFile | Out-Null

	$serverJob = Start-Job -ScriptBlock {
		cargo run --example server *> $null
	}

	$serverId = $serverJob.Id
	
	if (($null -eq $serverId) -or ($serverId -eq 0)) {
		$red = @{ ForegroundColor = 'Red'; }
		Write-Host @red "Unable to start a job for the server. Exiting."
		Remove-MyTestDebris $null
	}
	else {
		return $serverId
	}
}

function Initialize-MyConnection {
	Start-Sleep -Seconds 2

	$initConnectParams = @{
		Uri = "http://127.0.0.1:7878/"
		Method = 'Get'
		SkipHttpErrorCheck = $true
		ErrorAction = 'Ignore'
	}
	
	$stillConnecting = $true

	while ($stillConnecting) {
		Write-Host "Waiting for server..."

		$res = $null
		$res = (Invoke-WebRequest @initConnectParams).StatusCode

		if ($res -eq 200) {
			Write-Host "Server is live!`n"
			$stillConnecting = $false
		}
	}
}

function Remove-MyTestDebris {
	param(
		[Parameter(Mandatory = $true)]
		[System.Int32]$serverId
	)

	if ($null -ne $serverId) {
		$serverJob = (Get-Job | Where-Object { $_.Id -eq $serverId })
		$jobExists = [System.Boolean]$serverJob.Count

		if ($jobExists) {
            Remove-Job -Id $serverId -Force
		}

		cargo clean *> $null

		if ($LASTEXITCODE -ne 0) {
			$red = @{ ForegroundColor = 'Red'; }
			Write-Host @red "Unable to remove artifacts from this test."
		}
	}

	exit
}

function Get-MyFinalResult {
	$red = @{ ForegroundColor = 'Red'; }
	$green = @{ ForegroundColor = 'Green'; }
	$yellow = @{ ForegroundColor = 'Yellow'; }
	$cyan = @{ ForegroundColor = 'Cyan'; }

	if (!(Test-Path -Path $outputFile)) {
		Write-Host @red "Cannot locate the test output file. Exiting."
		return
	}
	
	if (!(Test-Path -Path $expectedFile)) {
		Write-Host @red "Cannot locate the expected output file. Exiting."
		return
	}

	$test = Get-Content -Path $outputFile -Raw -Encoding utf8
	$test = $test.Trim()
	
	if ([System.String]::IsNullOrEmpty($test)) {
		Write-Host @red "Test output file is empty. Exiting"
		return
	}

	$expected = Get-Content -Path $expectedFile -Raw -Encoding utf8
	$expected = $expected.Trim()

	if ([System.String]::IsNullOrEmpty($expected)) {
		Write-Host @red "Expected output file is empty. Exiting"
		return
	}

	if ($test -ceq $expected) {
		Write-Host @green "✔ ALL TESTS PASSED! \o/"
	}
	else {
		Write-Host @red "✗ THERE WERE TEST FAILURES :-("
		Write-Host @yellow "`n[EXPECTED OUTPUT]:`n$expected"
		Write-Host @cyan "`n[TEST OUTPUT]:`n$test"
	}
}

function Test-OneRoute {
    param(
		[Parameter(Mandatory = $true)]
		[System.String]$name,

		[Parameter(Mandatory = $true)]
		[System.String]$uri,

		[Parameter(Mandatory = $true)]
		[System.Int32]$serverId

	)

	$target = "http://127.0.0.1:7878${uri}"

	$connectParams = @{
		Uri = $target
		Method = 'Get'
		SkipHttpErrorCheck = $true
	}
	
	$res = (Invoke-WebRequest @connectParams).RawContent

	if ([System.String]::IsNullOrEmpty($res)) {
		$red = @{ ForegroundColor = 'Red'; }
		Write-Host @red "✗ THERE WERE TEST FAILURES :-("
		Write-Host @red "No response received for test: $name"
		Remove-MyTestDebris $serverId
	}
	else {
		$res -split '\r?\n' |
			Select-Object -First 4 |
			ForEach-Object { "$($_.Trim())" } |
			Join-String -Separator "`r`n" |
			Out-File -FilePath $outputFile -Encoding utf8 -Append
	}
}

function Test-MyServer {
	Set-Location -Path $crateDir

	Initialize-MyServer
	$serverId = Start-MyServer
	Initialize-MyConnection

	if ($serverId -ne 0) {
		$srvId = @{ ServerId = $serverId }
		Test-OneRoute -Name "Get index page test" -Uri "/" @srvId
		Test-OneRoute -Name "Get about page test" -Uri "/about" @srvId
		Test-OneRoute -Name "Get non-existent page test" -Uri "/foo" @srvId
		Test-OneRoute -Name "Get favicon icon test" -Uri "/favicon.ico" @srvId

		Get-MyFinalResult
		Remove-MyTestDebris $serverId
	}
	else {
		Remove-MyTestDebris $null
	}
}

Test-MyServer
