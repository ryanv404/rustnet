
$crateDir = ($PSScriptRoot | Get-Item).Parent
$serverFile = Join-Path -Path $crateDir -ChildPath `
	'target\debug\examples\server.exe'
$testOutputFile = Join-Path -Path $crateDir -ChildPath `
	'server_tests\test_output.txt'
$expectedOutputFile = Join-Path -Path $crateDir -ChildPath `
	'server_tests\expected_output.txt'

function Initialize-MyServer {
	if (Test-Path -Path $serverFile -PathType Leaf) {
		Write-Output "Removing prior build artifacts."

		$cleanJob = Start-Job -ScriptBlock { cargo clean *> $null }

		Wait-Job -Job $cleanJob | Out-Null
		Remove-Job -Job $cleanJob -Force | Out-Null
	}

	$buildJob = Start-Job -ScriptBlock {
		cargo build --example server *> $null
	}

	Wait-Job -Job $buildJob | Out-Null
	Remove-Job -Job $buildJob -Force | Out-Null
}

function Start-MyServer {
    if (Test-Path -Path $testOutputFile -PathType Leaf) {
        Remove-Item -Path $testOutputFile | Out-Null
	}

    New-Item -ItemType File -Path $testOutputFile *> $null

    if (Test-Path -Path $serverFile -PathType Leaf) {
		# It's faster to launch the binary directly.
		$serverJob = Start-Job -ScriptBlock {
			$serverFile = $serverFile
			Invoke-Expression $serverFile *> $null
		}

		$serverId = $serverJob.Id

		if (($null -eq $serverId) -or ($serverId -eq 0)) {
			Write-Error `
				"Unable to start the test server. Exiting now."
			Remove-MyTestDebris $null
		} else {
			$serverId
		}
	} else {
		Write-Error "Could not locate the server executable file. Exiting."
		Remove-MyTestDebris $null
	}
}

function Save-MyResult {
    param(
		[Parameter(Mandatory = $true)]
		[System.String]$name,

		[Parameter(Mandatory = $true)]
		[System.String]$result
	)

	$result | Out-File -FilePath $testOutputFile -Append
}

function Remove-MyTestDebris {
	param(
		[Parameter(Mandatory = $true)]
		[System.Int32]$serverId
	)

	if (($null -ne $serverId) -and ($serverId -ne 0)) {
		# Avoid raising an exception if job does not exist by filtering.
		$jobExists = $(
			Get-Job | Where-Object { $_.Id -eq $serverId } | Out-String
		)

		$jobExists = [System.Boolean]$jobExists.Length

		if ($jobExists) {
            Remove-Job -Id $serverId -Force
		}

		Write-Output "The test server with Job ID $serverId has been closed."
	}

	Write-Output "Finishing clean up and then exiting."

	if (Test-Path -Path $serverFile -PathType Leaf) {
		$cleanJob = Start-Job -ScriptBlock { cargo clean *> $null }

		Wait-Job -Job $cleanJob | Out-Null
		Remove-Job -Job $cleanJob -Force
	}

	exit
}

function Get-MyFinalResult {
	if (!(Test-Path -Path $testOutputFile -PathType Leaf) -or `
		!(Test-Path -Path $expectedOutputFile -PathType Leaf))
	{
		Write-Error "Cannot locate one or both test output files. Exiting."
		return
	}

	$test = Get-Content -Path $testOutputFile
	$expect = Get-Content -Path $expectedOutputFile

	if (($test.Length -eq 0) -or ($expect.Length -eq 0)) {
		Write-Error "✗ THERE WERE TEST FAILURES :-("
		Write-Error "One or both test output files were empty."
	}
	else {
		# This cmdlet will only output differences.
		$result = Compare-Object -ReferenceObject $test -DifferenceObject $expect

		# If there are zero differences, then the test output was as expected.
		if ($result.Count -eq 0) {
			Write-Host -ForegroundColor Green "`n✔ ALL TESTS PASSED! \o/`n"
		}
		else {
			Write-Host -ForegroundColor Red "`n✗ THERE WERE TEST FAILURES :-(`n"
		}
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

	$addr = 'http://127.0.0.1:7878'
	$uri = "$addr$uri"
	$res = (Invoke-WebRequest -Method 'Get' -Uri $uri -SkipHttpErrorCheck).RawContent

	if ([System.String]::IsNullOrEmpty($res)) {
		Write-Error "✗ THERE WERE TEST FAILURES :-("
		Write-Error "No response received for test: $name"

		Remove-MyTestDebris $serverId
	}
	else {
		$result = $res -split '\r?\n' |
			Select-Object -First 4 |
			ForEach-Object { "${_}`n" } |
			Out-String -NoNewline
		$result = $result.Trim()

		Save-MyResult $name $result
	}
}

function Test-MyServer {
	Set-Location -Path $crateDir

	Initialize-MyServer

	Write-Output "Launching the test server."
	$serverId = Start-MyServer

	Write-Output "The test server is running with Job ID ${serverId}."

	if ($serverId -ne 0) {
		Test-OneRoute -Name "Get index page test" -Uri "/" -ServerId $serverId
		Test-OneRoute -Name "Get about page test" -Uri "/about" -ServerId $serverId
		Test-OneRoute -Name "Get non-existent page test" -Uri "/foo" -ServerId $serverId
		Test-OneRoute -Name "Get favicon icon test" -Uri "/favicon.ico" -ServerId $serverId

		Get-MyFinalResult
		Remove-MyTestDebris $serverId
	}
	else {
		Remove-MyTestDebris $null
	}
}

Test-MyServer
