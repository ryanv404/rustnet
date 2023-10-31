# Create a dictionary to track the test results.
$tracker = New-Object 'System.Collections.Generic.Dictionary[String, bool]'

function Remove-OldCargoJobsIfPresent {
	$priorCargoJobs = Get-Job | Where-Object -FilterScript {
		$_.Command -match "cargo"
	}

	if ($priorCargoJobs.Count -ne 0) {
		foreach ($cargoJob in $priorCargoJobs) {
			Stop-Job -Job $cargoJob
			Remove-Job -Job $cargoJob
		}
	}
}

function Initialize-MyServer {
	$red = @{ ForegroundColor = 'Red'; }
	$yellow = @{ ForegroundColor = 'Yellow'; }

	Remove-OldCargoJobsIfPresent

	cargo clean *> $null

	if ($LASTEXITCODE -ne 0) {
		Write-Host @yellow "Unable to remove the prior build artifacts. Continuing."
	}

	Write-Host "Building..."

	cargo build --bin server *> $null

	if ($LASTEXITCODE -ne 0) {
		Write-Host @red "`nUnable to build the server. Exiting."
		exit
	}
}

function Start-MyServer {
	$serverJob = Start-Job -ScriptBlock {
		cargo run --bin server *> $null
	}

	if (($null -eq $serverJob.Id) -or ($serverJob.Id -eq 0)) {
		$red = @{ ForegroundColor = 'Red'; }
		Write-Host @red "`nUnable to start a job to run the server. Exiting."
		Remove-MyTestDebris
	}
}

function Initialize-MyConnection {
	$red = @{ ForegroundColor = 'Red'; }
	$green = @{ ForegroundColor = 'Green'; }

	$attemptNum = 0
	$maxAttempts = 5
	$stillConnecting = $true

	$initConnectParams = @{
		Method = 'Get'
		Uri = 'http://127.0.0.1:7878/'
		SkipHttpErrorCheck = $true
	}
	
	do {
		Write-Host -NoNewline "Checking if server is live..."
	
		try {
			$res = $null
			$res = (Invoke-WebRequest @initConnectParams -ErrorAction Stop).StatusCode
		}
		catch {
			Write-Host @red "Got an exception."
			Write-Host "$($_.Exception.Message)"
		}
		
		if ($res -eq 200) {
			Write-Host @green "Server is live!`n"
			$stillConnecting = $false
			return
		}
		else {
			$attemptNum++
		}

	} while (($attemptNum -lt $maxAttempts) -and ($stillConnecting))

	if ($stillConnecting) {
		Write-Host @red "`nServer is unreachable. Exiting."
		Remove-MyTestDebris
	}
}

function Remove-MyTestDebris {
	Remove-OldCargoJobsIfPresent
	cargo clean *> $null
	exit
}

function Test-OneRoute {
    param(
		[Parameter(Mandatory = $true)]
		[System.String]$testName,

		[Parameter(Mandatory = $true)]
		[System.String]$uri
	)

	$red = @{ ForegroundColor = 'Red'; }
	$green = @{ ForegroundColor = 'Green'; }
	$blue = @{ ForegroundColor = 'Blue'; }

	$joinParams = @{
		Path = $crateDir
		ChildPath = 'scripts'
		AdditionalChildPath = "${testName}.txt"
	}

	$expectedOutputFile = Join-Path @joinParams

	Write-Host "[" -NoNewline
	Write-Host @blue $testName -NoNewline
	Write-Host "]: " -NoNewline

	if (!(Test-Path -Path $expectedOutputFile)) {
		Write-Host @red "✗ (No expected output file found for this test)."
		$tracker.Add($testName, $false)
		return
	}
	
	$getContentParams = @{
		Path = $expectedOutputFile
		Encoding = "utf8"
		Raw = $true
	}
	
	$expectedOutput = (Get-Content @getContentParams).Trim()
	
	if ([System.String]::IsNullOrEmpty($expectedOutput)) {
		Write-Host @red "✗ (The expected output file for this test is empty)."
		$tracker.Add($testName, $false)
		return
	}
	
	$connectParams = @{
		Method = 'Get'
		Uri = "http://127.0.0.1:7878${uri}"
		SkipHttpErrorCheck = $true
	}
	
	try {
		$res = $null
		$res = (Invoke-WebRequest @connectParams).RawContent
		
		if ([System.String]::IsNullOrEmpty($res)) {
			Write-Host @red "✗ (No response received)."
			$tracker.Add($testName, $false)
			return
		}
		
		$testOutput = $res -split '\r?\n' |
		Select-Object -First 4 |
		Join-String -Separator "`r`n"
		
		$testOutput = $testOutput.Trim()
		
		if ($testOutput -ceq $expectedOutput) {
			Write-Host @green "✔"
			$tracker.Add($testName, $true)
		} else {
			Write-Host @red "✗ (Did not match the expected output)."
			$tracker.Add($testName, $false)
		}
	}
	catch {
		Write-Host @red "✗ (Connection error. $($_.Exception.Message))."
		$tracker.Add($testName, $false)
	}
}

function Test-MyServer {
	$crateDirParams = @{
		Name = "crateDir"
		Option = "AllScope", "Constant"
		Value = (($PSScriptRoot | Get-Item).Parent)
	}

	New-Variable @crateDirParams
	Set-Location -Path $crateDir

	Initialize-MyServer
	Start-Sleep -Seconds 2

	Start-MyServer
	Initialize-MyConnection

	Test-OneRoute -TestName "get_index_headers" -Uri "/"
	Test-OneRoute -TestName "get_about_headers" -Uri "/about"
	Test-OneRoute -TestName "get_non_existent_page_headers" -Uri "/foo"
	Test-OneRoute -TestName "get_favicon_headers" -Uri "/favicon.ico"

	Get-MyFinalResult
	Remove-MyTestDebris
}

function Get-MyFinalResult {
	$red = @{ ForegroundColor = 'Red'; }
	$green = @{ ForegroundColor = 'Green'; }

	$numPassed = 0
	$totalTests = $tracker.Count

	$tracker.Values | ForEach-Object {
		if ($_) {
			$numPassed++
		}
	}

	if ($numPassed -eq $totalTests) {
		Write-Host @green "`n$numPassed / $totalTests tests passed."
	}
	else {
		Write-Host @red "`n$numPassed / $totalTests tests passed."
	}
}

# Run all tests.
Test-MyServer
