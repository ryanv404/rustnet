# Script-level variables to track testing results.
$script:numTests = 0
$script:numPassed = 0

# Builds the test server.
function Build-MyServer {
	$red = @{ ForegroundColor = 'Red'; }
	$green = @{ ForegroundColor = 'Green'; }
	$yellow = @{ ForegroundColor = 'Yellow'; }

	Remove-OldCargoJobs

	cargo clean *> $null

	if ($LASTEXITCODE -ne 0) {
		Write-Host @yellow "Unable to remove the prior build artifacts."
	}

	Write-Host -NoNewline "Building..."

	cargo build --bin server *> $null

	if ($LASTEXITCODE -ne 0) {
		Write-Host @red "✗ Unable to build the server."
		exit
	}
    else {
        Write-Host @green "✔"
    }
}

# Starts the test server as a background job.
function Start-MyServer {
    $red = @{ ForegroundColor = 'Red'; }

    Build-MyServer

    $crateDir = ($PSScriptRoot | Get-Item).Parent

    $joinParams = @{
        Path = $crateDir
        ChildPath = 'target'
        AdditionalChildPath = 'debug', 'server.exe'
    }

    $serverExe = Join-Path @joinParams

	if (!(Test-Path -Path $serverExe)) {
		Write-Host @red "`n✗ Cannot locate the server executable file."
        Remove-BuildArtifacts
    }

    $serverJob = Start-Job -ScriptBlock { & $using:serverExe *>&1 }

    # Pause briefly to allow the server to start up.
    Start-Sleep -Seconds 1

    if (($null -eq $serverJob.Id) -or ($serverJob.Id -eq 0)) {
		Write-Host @red "✗ Unable to start the server in a background job."
		Remove-BuildArtifacts
	}

    Confirm-MyServerIsLive
}

# Confirms the server is live and reachable.
function Confirm-MyServerIsLive {
	$red = @{ ForegroundColor = 'Red'; }
	$green = @{ ForegroundColor = 'Green'; }

	$attemptNum = 0
	$maxAttempts = 5
	$stillNotLive = $true

	$initConnectParams = @{
		Method = 'Get'
		Uri = 'http://127.0.0.1:7878/'
		SkipHttpErrorCheck = $true
        ErrorAction = 'Stop'
    }

	do {
		Write-Host -NoNewline "Connecting..."

		try {
			$statusCode = (Invoke-WebRequest @initConnectParams).StatusCode
		}
		catch {
			Write-Host @red "✗ Connection error."
			Write-Host @red $_.Exception.Message
		}

		if ($statusCode -eq 200) {
			Write-Host @green "✔`n"
			$stillNotLive = $false
			return
		}
		else {
			$attemptNum++
		}

	} while (($attemptNum -lt $maxAttempts) -and ($stillNotLive))

	if ($stillNotLive) {
		Write-Host @red "`n✗ The server is unreachable."
		Remove-BuildArtifacts
	}
}

# Removes any background Cargo jobs that may be running.
function Remove-OldCargoJobs {
	$priorCargoJobs = Get-Job | Where-Object -FilterScript {
		(($_.Command -like "*cargo*") -or ($_.Command -like "*serverExe*"))
	}

	if ($priorCargoJobs.Count -ne 0) {
        $serverLog = Join-Path -Path $PSScriptRoot -ChildPath 'server.log'

        New-Item -Path $serverLog -ItemType File -Force | Out-Null

        foreach ($cargoJob in $priorCargoJobs) {
            (Receive-Job -Job $cargoJob) |
                Where-Object { $_ -is [string] } |
                Out-File -FilePath $serverLog

            Stop-Job -Job $cargoJob
			Remove-Job -Job $cargoJob
		}
	}
}

# Remove old build artifacts.
function Remove-BuildArtifacts {
	Remove-OldCargoJobs
	cargo clean *> $null
	exit
}

# Writes a message to indicate the test passed.
function Write-TestPassed {
    param(
		[Parameter(Mandatory = $true)]
		[string]
        $label
    )

    $blue = @{ ForegroundColor = 'Blue'; }
    $green = @{ ForegroundColor = 'Green'; }

    Write-Host -NoNewline "["
    Write-Host -NoNewline @green "✔"
    Write-Host -NoNewline "] "
    Write-Host @blue $label
}

# Writes a message to indicate the test failed.
function Write-TestFailed {
    param(
		[Parameter(Mandatory = $true)]
		[string]
        $label,

		[Parameter(Mandatory = $true)]
		[string]
        $reason
    )

    $red = @{ ForegroundColor = 'Red'; }
    $blue = @{ ForegroundColor = 'Blue'; }

    Write-Host -NoNewline "["
    Write-Host -NoNewline @red "✗"
    Write-Host -NoNewline "] "
    Write-Host -NoNewline @blue $label
    Write-Host -NoNewline ": "
    Write-Host @red $reason
}

# Runs a single test.
function Test-OneRoute {
    param(
		[Parameter(Mandatory = $true)]
		[string]
        $method,

        [Parameter(Mandatory = $true)]
		[string]
        $uri,

        [Parameter(Mandatory = $true)]
		[string]
        $label,

        [Parameter(Mandatory = $true)]
		[string]
        $file
    )

    $expOutputFile = Join-Path -Path $PSScriptRoot -ChildPath 'server_tests' -AdditionalChildPath $file

	if (!(Test-Path -Path $expOutputFile)) {
        Write-TestFailed $label "No expected output file found."
		return
	}

	$expOutput = Get-Content -Path $expOutputFile -Encoding 'utf8' -Raw
    $expOutput = $expOutput -split "`n" |
        ForEach-Object { $_.Trim() }

    if (($null -eq $expOutput) -or ($expOutput.Count -eq 0)) {
        Write-TestFailed $label "The expected output file was empty."
        return
    }

	try {
        if ($method -eq "CONNECT") {
            $headers = @{
                'Host' = '127.0.0.1'
            }

            $connectParams = @{
                CustomMethod = $method
                Headers = $headers
                Uri = $uri
                SkipHttpErrorCheck = $true
            }
        }
        else {
            $connectParams = @{
                Method = $method
                Uri = "http://127.0.0.1:7878${uri}"
                SkipHttpErrorCheck = $true
            }
        }

        $res = Invoke-WebRequest @connectParams
    }
    catch {
        Write-TestFailed $label "Connection error.`n$($_.Exception.Message)"
        return
    }

    if ($null -eq $res) {
        Write-TestFailed $label "No response received."
        return
    }

    $expStatusLine = ($expOutput.Count -gt 1) ? $expOutput[0] : $expOutput

    $statusLineParams = @{
        Label = $label
        BaseResponse = $res.BaseResponse
        StatusCode = $res.StatusCode
        ExpStatusLine = $expStatusLine
    }

    $testPassed = $false
    $testPassed = Test-StatusLine @statusLineParams

    if ($testPassed -eq $false) {
        return
    }

    # IndexOf returns -1 if the string cannot be found.
    $HeadersEnd = $expOutput.IndexOf("")

    if ($HeadersEnd -eq -1) {
        Write-TestFailed $label "End of expected headers section not found."
        return
    }

    # If headers end at line index 1 then there are no headers and no body.
    if ($HeadersEnd -gt 1) {
        [string[]] $testHeaders = ${res}?.Headers.GetEnumerator() |
            ForEach-Object { "$($_.Key): $($_.Value?[0])" }

        [string[]] $expHeaders = $expOutput[1..($HeadersEnd - 1)]

        $headersParams = @{
            Label = $label
            TestHeaders = $testHeaders
            ExpHeaders = $expHeaders
        }

        $testPassed = $false
        $testPassed = Test-ResponseHeaders @headersParams

        if ($testPassed -eq $false) {
            return
        }

        $contentType = ${res}.Headers?["Content-Type"]?[0]

        # Only test the body if appropriate.
        if (($method -ne 'Head') -and
            ($null -ne $contentType) -and
            ($contentType -notlike "*image*"))
        {
            $bodyStart = $HeadersEnd + 1
            $bodyEnd = $expOutput.Count - 1

            $expBody = $expOutput[($bodyStart)..($bodyEnd)] |
                Join-String -Separator "`n"

            $testBody = $res.Content -split "`n" |
                ForEach-Object { $_.Trim() } |
                Join-String -Separator "`n"

            $bodyParams = @{
                Label = $label
                TestBody = $testBody
                ExpBody = $expBody
            }

            $testPassed = $false
            $testPassed = Test-ResponseBody @bodyParams

            if ($testPassed -eq $false) {
                return
            }
        }
    }

    # Only tests that have not failed a prior step make it to this point.
    Write-TestPassed $label
    $script:numPassed++
}

# Test whether the output status line matches the expected status line.
function Test-StatusLine {
    param (
        [Parameter(Mandatory = $true)]
        [string]
        $label,

        [Parameter(Mandatory = $true)]
        [System.Net.Http.HttpResponseMessage]
        $baseResponse,

        [Parameter(Mandatory = $true)]
        [Int32]
        $statusCode,

        [Parameter(Mandatory = $true)]
        [string]
        $expStatusLine
    )

    # Color settings for Write-Host
	$yellow = @{ ForegroundColor = 'Yellow'; }
	$magenta = @{ ForegroundColor = 'Magenta'; }

    $version = $baseResponse.Version
    $statusMsg = $baseResponse.ReasonPhrase
    $testStatusLine = "HTTP/$($version) $($statusCode) $($statusMsg)"

    if ($testStatusLine -cne $expStatusLine) {
        Write-TestFailed $label "Did not match the expected status line."
        Write-Host @yellow "`n[EXPECTED] $expStatusLine"
        Write-Host @magenta "[OUTPUT] $testStatusLine`n"
        return $false
    }
    else {
        return $true
    }
}

# Test whether the output headers match the expected headers.
function Test-ResponseHeaders {
    param (
        [Parameter(Mandatory = $true)]
        [string]
        $label,

        [Parameter(Mandatory = $true)]
        [string[]]
        $testHeaders,

        [Parameter(Mandatory = $true)]
        [string[]]
        $expHeaders
    )

    # Color settings for Write-Host
	$yellow = @{ ForegroundColor = 'Yellow'; }
	$magenta = @{ ForegroundColor = 'Magenta'; }

    if ($null -eq $testHeaders) {
        Write-TestFailed $label "Response does not contain any headers."
        return $false
    }

    if ($testHeaders.Count -ne $expHeaders.Count) {
        Write-TestFailed $label "Incorrect number of headers."
        Write-Host @yellow "`n[EXPECTED TOTAL] $($expHeaders.Count)"
        Write-Host @magenta "[OUTPUT TOTAL] $($testHeaders.Count)`n"
        return $false
    }

    foreach ($idx in 0..$expHeaders.Count) {
        $expHdr = $expHeaders[$idx]
        $testHdr = $testHeaders[$idx]

        if ($testHdr -cne $expHdr) {
            Write-TestFailed $label "Did not match the expected headers."
            Write-Host @yellow "`n[EXPECTED] $expHdr"
            Write-Host @magenta "[OUTPUT] $testHdr`n"
            return $false
        }
    }

    return $true
}

# Test whether the output body matches the expected body, if applicable.
function Test-ResponseBody {
    param (
        [Parameter(Mandatory = $true)]
        [string]
        $label,

        [Parameter(Mandatory = $true)]
        [string]
        $testBody,

        [Parameter(Mandatory = $true)]
        [string]
        $expBody
    )

    # Color settings for Write-Host
	$yellow = @{ ForegroundColor = 'Yellow'; }
	$magenta = @{ ForegroundColor = 'Magenta'; }

    if ($testBody -cne $expBody) {
        Write-TestFailed $label "Did not match the expected body."
        Write-Host @magenta "`n[OUTPUT]`n$testBody"
        Write-Host @yellow "`n[EXPECTED]`n$expBody"
        return $false
    }
    else {
        return $true
    }
}

# Runs all server tests and reports the results.
function Test-MyServer {
	$red = @{ ForegroundColor = 'Red'; }
	$blue = @{ ForegroundColor = 'Blue'; }
	$green = @{ ForegroundColor = 'Green'; }

    $border = "+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+"

    Start-MyServer

    $tests = Get-ChildItem -Path "${PSScriptRoot}/server_tests/*.txt" -File |
        ForEach-Object { $_.BaseName }

    # Parse the test parameters from each test's file name and run the test.
    foreach ($test in $tests) {
        $script:numTests++

        $parts = $test -split "_"
        $method = $parts[0].ToUpper()

        $uri = switch ($parts[1]) {
            "index"   { "/"; Break }
            "favicon" { "/favicon.ico"; Break }
            Default   {
                if ($method -eq "CONNECT") {
                    "$($parts[1])"
                }
                else {
                    "/$($parts[1])"
                }
            }
        }

        $label = "$method $uri"
        $file = "${test}.txt"

        Test-OneRoute $method $uri $label $file
    }

    # Write the overall results to the terminal.
    Write-Host @blue "`n$border"

    if ($script:numPassed -eq $script:numTests) {
        Write-Host @green "$script:numPassed / $script:numTests tests passed."
	}
	else {
        Write-Host @red "$script:numPassed / $script:numTests tests passed."
	}

    Write-Host @blue $border

    Remove-BuildArtifacts
}

# Run all server tests.
Test-MyServer
