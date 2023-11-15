# Test 1: GET /
$test1Params = @{ TestName = "get_index"; Uri = "/"; }
# Test 2: GET /
$test2Params = @{ TestName = "get_about"; Uri = "/about"; }
# Test 3: GET /
$test3Params = @{ TestName = "get_foo"; Uri = "/foo"; }
# Test 4: GET /
$test4Params = @{ TestName = "get_favicon"; Uri = "/favicon.ico"; }

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

function Start-OneTest {
    param(
		[Parameter(Mandatory = $true)]
		[System.String]$testName,

		[Parameter(Mandatory = $true)]
		[System.String]$uri
	)

    # Initialize variables that will use null testing.
    $res = $null
    $testHeaders = $null
    $testContentType = $null
    $expectedOutput = $null

    # Color settings for Write-Host
    $red = @{ ForegroundColor = 'Red'; }
	$blue = @{ ForegroundColor = 'Blue'; }
	$green = @{ ForegroundColor = 'Green'; }
	$yellow = @{ ForegroundColor = 'Yellow'; }
	$magenta = @{ ForegroundColor = 'Magenta'; }

    # Construct the path "${crateDir}\scripts\tests\${testName}.txt"
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
		Write-Host @red "✗ No expected output file found."
		$tracker.Add($testName, $false)
		return
	}

	$expectedOutput = Get-Content -Path $expectedOutputFile -Encoding 'utf8' |
        ForEach-Object { $_.Trim() }

    if (($null -eq $expectedOutput) -or ($expectedOutput.Count -eq 0)) {
        Write-Host @red "✗ The expected output file was empty."
        $tracker.Add($testName, $false)
        return
    }

	try {
        # We want to analyze 4xx and 5xx responses so set SkipHttpErrorCheck
        $connectParams = @{
            Method = 'Get'
            Uri = "http://127.0.0.1:7878${uri}"
            SkipHttpErrorCheck = $true
        }

        $res = Invoke-WebRequest @connectParams
    }
    catch {
        Write-Host @red "✗ Connection error. $($_.Exception.Message)."
        $tracker.Add($testName, $false)
    }

    if ($null -eq $res) {
        Write-Host @red "✗ No response received."
        $tracker.Add($testName, $false)
        return
    }

    # Test whether the output status line matches the expected status line.
    #
    # $res changes the status message from 'Not Found' to 'NotFound' which
    # messes with the upcoming comparisons. So I'll just rebuild all status
    # lines from scratch using the BaseResponse, which contains exactly what
    # the server sent back.
    $version = $res.BaseResponse.Version
    $statusCode = $res.StatusCode
    $statusMsg = $res.BaseResponse.ReasonPhrase

    $testStatusLine = "HTTP/$($version) $($statusCode) $($statusMsg)"
    $expectedStatusLine = $expectedOutput[0]

    if (!($testStatusLine -ceq $expectedStatusLine)) {
        Write-Host @red "✗ Did not match the expected status line."
        Write-Host @yellow "`n[EXPECTED] $expectedStatusLine"
        Write-Host @magenta "[OUTPUT] $testStatusLine`n"
        $tracker.Add($testName, $false)
        return
    }

    # Test whether the output headers match the expected headers.
    $testHeaders = ${res}?.Headers

    if ($null -eq $testHeaders) {
        Write-Host @red "✗ Response does not contain any headers."
        $tracker.Add($testName, $false)
        return
    }

    $testContentType = ${testHeaders}?["Content-Type"]?[0]

    if ($null -eq $testContentType) {
        Write-Host @red "✗ Response does not contain a Content-Type header."
        $tracker.Add($testName, $false)
        return
    }

    # IndexOf returns -1 if the string cannot be found.
    $blankLineIndex = $expectedOutput.IndexOf("")

    # We expect the expected headers to start at index 1.
    if ($blankLineIndex -le 1) {
        Write-Host @red "✗ Expected headers are formatted incorrectly."
        Write-Host @yellow "`nEnsure that the expected headers section starts `
            on the 2nd line and ends with a blank line.`n"
        $tracker.Add($testName, $false)
        return
    }

    # The index of the first blank line is greater than 1.
    $expectedHeaders = $expectedOutput[1..($blankLineIndex - 1)]

    $testHeaders = $testHeaders.GetEnumerator() |
        ForEach-Object { "$($_.Key): $($_.Value)" }

    if ($testHeaders.Count -ne $expectedHeaders.Count) {
        Write-Host @red "✗ Did not match the expected number of headers."
        Write-Host @yellow "`n[EXPECTED TOTAL] $($expectedHeaders.Count)"
        Write-Host @magenta "[OUTPUT TOTAL] $($testHeaders.Count)`n"
        $tracker.Add($testName, $false)
        return
    }

    foreach ($idx in 0..$expectedHeaders.Count) {
        $expHdr = $expectedHeaders[$idx]
        $testHdr = $testHeaders[$idx]

        if (!($testHdr -ceq $expHdr)) {
            Write-Host @red "✗ Did not match the expected headers."
            Write-Host @yellow "`n[EXPECTED] $expHdr"
            Write-Host @magenta "[OUTPUT]   $testHdr`n"
            $tracker.Add($testName, $false)
            return
        }
    }

    # Test whether the output body matches the expected body, if applicable.
    if ($testContentType.Contains("text")) {
        $testBody = $res.Content -split "`r?`n" |
            ForEach-Object { $_.Trim() } |
            Where-Object { $_.Length -gt 0 } |
            Join-String -Separator "`r`n"

        $expectedEnd = $expectedOutput.Count - 1
        $expectedBody = $expectedOutput[($blankLineIndex + 1)..($expectedEnd)] |
            ForEach-Object { $_.Trim() } |
            Join-String -Separator "`r`n"

        if (!($testBody -ceq $expectedBody)) {
            Write-Host @red "✗ Did not match the expected body."
            Write-Host @magenta "`n[OUTPUT]`n$testBody"
            Write-Host @yellow "`n[EXPECTED]`n$expectedBody"
            $tracker.Add($testName, $false)
            return
        }
    }

    # All tests have passed at this point.
    Write-Host @green "✔"
    $tracker.Add($testName, $true)
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

	Start-OneTest @test1Params
	Start-OneTest @test2Params
	Start-OneTest @test3Params
	Start-OneTest @test4Params

	Get-MyFinalResult
	Remove-MyTestDebris
}

function Get-MyFinalResult {
	$red = @{ ForegroundColor = 'Red'; }
	$blue = @{ ForegroundColor = 'Blue'; }
	$green = @{ ForegroundColor = 'Green'; }

    $border = "+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+"

	$numPassed = 0
	$totalTests = $tracker.Count

	$tracker.Values | ForEach-Object {
        if ($_) {
            $numPassed++
		}
	}

    Write-Host @blue "`n$border`n"

    if ($numPassed -eq $totalTests) {
        Write-Host @green "$numPassed / $totalTests tests passed.`n"
	}
	else {
		Write-Host @red "$numPassed / $totalTests tests passed.`n"
	}
}

# Run all tests.
Test-MyServer
