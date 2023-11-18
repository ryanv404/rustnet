# Script-level variables to track testing results.
$script:numServerTests = 0
$script:numClientTests = 0
$script:numServerPassed = 0
$script:numClientPassed = 0

# Builds the client.
function Build-MyClient {
	$red = @{ ForegroundColor = 'Red'; }
	$green = @{ ForegroundColor = 'Green'; }
	$yellow = @{ ForegroundColor = 'Yellow'; }

	cargo clean *> $null

	if ($LASTEXITCODE -ne 0) {
		Write-Host @yellow "Unable to remove the prior build artifacts."
	}

	Write-Host -NoNewline "Building client..."

	cargo build --bin client *> $null

	if ($LASTEXITCODE -ne 0) {
		Write-Host @red "✗ Unable to build the client."
		exit
	}
    else {
        Write-Host @green "✔`n"
    }
}

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

	Write-Host -NoNewline "Building server..."

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
		Uri = '127.0.0.1:7878/'
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

# Runs a single server test.
function Test-OneServerRoute {
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

	$joinParams = @{
		Path = $PSScriptRoot
		ChildPath = 'server_tests'
		AdditionalChildPath = $file
	}

    $expOutputFile = Join-Path @joinParams

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
                Uri = "127.0.0.1:7878${uri}"
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
    $testPassed = Test-ResponseStatusLine @statusLineParams

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
    $script:numServerPassed++
}

# Test whether the output status line matches the expected status line.
function Test-ResponseStatusLine {
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

# Runs a single client test.
function Test-OneClientRoute {
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

	$joinParams = @{
		Path = $PSScriptRoot
		ChildPath = 'client_tests'
		AdditionalChildPath = $file
	}

	$expOutputFile = Join-Path @joinParams

	if (!(Test-Path -Path $expOutputFile)) {
        Write-TestFailed $label "No expected output file found."
		return
	}

	$expOutput = Get-Content -Path $expOutputFile -Encoding 'utf8' -Raw

    if ($null -eq $expOutput) {
        Write-TestFailed $label "The expected output file was empty."
        return
    }

	$crateDir = ($PSScriptRoot | Get-Item).Parent

    $joinParams = @{
        Path = $crateDir
        ChildPath = 'target'
        AdditionalChildPath = 'debug', 'client.exe'
    }

    $clientExe = Join-Path @joinParams

	if (!(Test-Path -Path $clientExe)) {
		Write-Host @red "`n✗ Cannot locate the client executable file."
        Remove-BuildArtifacts
    }

	$clientParams = @('--testing', 'httpbin.org', $uri)
	
	$clientJob = Start-Job -ScriptBlock {
		& $using:clientExe @using:clientParams *>&1
	}

	# Using Force ensures we wait until the job is in either the Completed,
	# Stopped, or Failed states.
	$res = (Receive-Job -Job $clientJob -Wait -Force)

	if ($null -eq $res) {
        Write-TestFailed $label "No response received."
        return
    }

    $expOutput = $expOutput -split "`n" |
        ForEach-Object { $_.Trim() } |
		Where-Object { $_.Length -gt 0 } |
		Join-String -Separator "`n"

	$testOutput = $res -split "`n" |
        ForEach-Object { $_.Trim() } |
		Where-Object { $_.Length -gt 0 } |
		Join-String -Separator "`n"

	if ($expOutput -ceq $testOutput) {
		Write-TestPassed $label
		$script:numClientPassed++
	}
	else {
		$charIdx = Compare-String $expOutput $testOutput

		Write-TestFailed $label "Did not match the expected output."
		Write-Host -NoNewline "Got ""$($testOutput[$charIdx])"" instead of "
		Write-Host """$($expOutput[$charIdx])"" at character number ${charIdx}."
	}
}

# Compares two strings, returning the index of the first non-equal character
# or -1 if the two strings are identical.
#
# https://stackoverflow.com/questions/25169424/using-powershell-to-find-the-differences-in-strings
function Compare-String {
	param(
		[string]
		$s1,

		[string]
		$s2
	)

	if ( $s1 -ceq $s2 ) {
		return -1
	}

	$maxLength = ( $s1, $s2 |
		ForEach-Object {$_.Length} |
		Measure-Object -Maximum ).Maximum

	for ( $i = 0; $i -lt $maxLength; $i++ ) {
		if ( $s1[$i] -cne $s2[$i] ) {
			return $i
		}
	}

	return $maxLength
}

# Runs all server tests and reports the results.
function Test-MyServer {
    Start-MyServer

    $tests = Get-ChildItem -Path "${PSScriptRoot}/server_tests/*.txt" -File |
        ForEach-Object { $_.BaseName }

	Write-Host "SERVER TESTS:"

	# Parse the test parameters from each test's file name and run the test.
    foreach ($test in $tests) {
        $script:numServerTests++

        $parts = $test -split "_"
        $method = $parts[0].ToUpper()

        $uri = switch ($($parts[1].ToLower())) {
            "index"   { "/"; Break }
            "favicon" { "/favicon.ico"; Break }
            Default   { "/$($parts[1])" }
        }

        $label = "$method $uri"
        $file = "${test}.txt"

        Test-OneServerRoute $method $uri $label $file
    }
}

# Runs all client tests and reports the results.
function Test-MyClient {
    Build-MyClient

    $tests = Get-ChildItem -Path "${PSScriptRoot}/client_tests/*.txt" -File |
        ForEach-Object { $_.BaseName }

	Write-Host "CLIENT TESTS:"

	# Parse the test parameters from each test's file name and run the test.
    foreach ($test in $tests) {
        $script:numClientTests++

        $parts = $test -split "_"
        $method = $parts[0].ToUpper()

		$uri = $parts[1].ToLower()
        $uri = switch ($uri) {
			"jpeg"  { "/image/jpeg"; Break }
			"png"   { "/image/png"; Break }
			"svg"   { "/image/svg"; Break }
			"text"  { "/robots.txt"; Break }
			"utf8"  { "/encoding/utf8"; Break }
			"webp"  { "/image/webp"; Break }
			Default { "/$uri" }
		}

		$label = "$method $uri"
        $file = "${test}.txt"

        Test-OneClientRoute $method $uri $label $file
    }
}

# Write the overall results to the terminal.
function Write-OverallResults {
	$red = @{ ForegroundColor = 'Red'; }
	$blue = @{ ForegroundColor = 'Blue'; }
	$green = @{ ForegroundColor = 'Green'; }

    $border = "+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+"

	$cTotal = $script:numClientTests
	$sTotal = $script:numServerTests
	
	if (($cTotal -eq 0) -and ($sTotal -eq 0)) {
		return
	}

	Write-Host @blue "`n$border"

	if ($cTotal -gt 0) {
		$cPassed = $script:numClientPassed
		$color = ($cPassed -eq $cTotal) ? $green : $red
		Write-Host @color "$cPassed / $cTotal client tests passed."
	}

	if ($sTotal -gt 0) {
		$sPassed = $script:numServerPassed
		$color = ($sPassed -eq $sTotal) ? $green : $red
		Write-Host @color "$sPassed / $sTotal server tests passed."
	}

    Write-Host @blue $border
}

# Writes a help message to the terminal.
function Write-MyHelp {
	$green = @{ ForegroundColor = 'Green'; }
	$progName = $MyInvocation.ScriptName | Split-Path -Leaf

	Write-Host @green "USAGE"
	Write-Host "    $progName <ARGUMENT>`n"
    Write-Host @green "ARGUMENTS"
    Write-Host "    all      Run all tests."
    Write-Host "    client   Run all client tests only."
    Write-Host "    server   Run all server tests only.`n"
}

# Handle command line arguments.
if ($args.Count -lt 1) {
	$red = @{ ForegroundColor = 'Red'; }
	Write-Host @red "Please select a test group to run.`n"
	Write-MyHelp
}
else {
	switch ($($args[0].ToLower())) {
		"client" {
			Test-MyClient
			Write-OverallResults
			Remove-BuildArtifacts
			Break
		}
		"server" {
			Test-MyServer
			Write-OverallResults
			Remove-BuildArtifacts
			Break
		}
		"all" {
			Test-MyClient
			Write-Host ""
			Test-MyServer
			Write-OverallResults
			Remove-BuildArtifacts
			Break
		}
		Default {
			$red = @{ ForegroundColor = 'Red'; }
			Write-Host @red "Unknown argument: \"$($args[0])\"`n"
			Write-MyHelp
		}
	}
}

exit
