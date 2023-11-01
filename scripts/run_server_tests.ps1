# Create a dictionary to track the test results.
$tracker = New-Object 'System.Collections.Generic.Dictionary[String, bool]'

function Remove-OldCargoJobsIfPresent {
	$priorCargoJobs = Get-Job | Where-Object -FilterScript {
		$_.Command -match "cargo"
	}

	if ($priorCargoJobs.Count -ne 0) {
        foreach ($cargoJob in $priorCargoJobs) {
            (Receive-Job -Job $cargoJob) |
                Where-Object { ($_.GetType()).Name -eq 'String' } |
                Out-File -FilePath $serverLog

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
		cargo run --bin server *>&1
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
        ErrorAction = 'Stop'
    }

	do {
		Write-Host -NoNewline "Connecting..."

		try {
			$res = $null
			$res = (Invoke-WebRequest @initConnectParams).StatusCode
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
	$yellow = @{ ForegroundColor = 'Yellow'; }
	$blue = @{ ForegroundColor = 'Blue'; }
	$magenta = @{ ForegroundColor = 'Magenta'; }

	$joinParams = @{
		Path = $crateDir
		ChildPath = 'scripts'
		AdditionalChildPath = 'tests', "${testName}.txt"
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
		$res = Invoke-WebRequest @connectParams

		if ([System.String]::IsNullOrEmpty($res)) {
			Write-Host @red "✗ (No response received)."
			$tracker.Add($testName, $false)
			return
		}

        if ($res.Headers["Content-Type"][0] -clike "image/x-icon") {
            $testOutput = $res.RawContent -split '\r?\n' |
                Select-Object -First 4 |
                Join-String -Separator "`r`n"
        }
        else {
            $testOutput = $res.RawContent
        }

		$testOutput = $testOutput.Trim()

		if ($testOutput -ceq $expectedOutput) {
			Write-Host @green "✔"
			$tracker.Add($testName, $true)
		} else {
			Write-Host @red "✗ (Did not match the expected output)."
            Write-Host @yellow "--[EXPECTED]--`n$expectedOutput"
            Write-Host @magenta "--[OUTPUT]--`n$testOutput"
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

    $serverLogJoinParams = @{
        Path = $crateDir
        ChildPath = 'scripts'
        AdditionalChildPath = 'server_log.txt'
    }

    $serverLogParams = @{
        Name = "serverLog"
		Option = "AllScope", "Constant"
		Value = (Join-Path @serverLogJoinParams)
    }

    New-Variable @serverLogParams

    Set-Location -Path $crateDir

	Initialize-MyServer
	Start-Sleep -Seconds 2

	Start-MyServer
	Initialize-MyConnection

	Test-OneRoute -TestName "get_index_win" -Uri "/"
	Test-OneRoute -TestName "get_about_win" -Uri "/about"
	Test-OneRoute -TestName "get_foo_win" -Uri "/foo"
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
