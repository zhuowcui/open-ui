<#
.SYNOPSIS
Drive the accessibility example through Windows UI Automation.

.DESCRIPTION
Launches the example built with the `windows` feature and acts as an
assistive client through the .NET UI Automation client. It checks that the
window exposes the Open UI framework, that named controls appear with their
UIA control types, and that focus, invoke, toggle, value and window-close
requests reach the native document and come back as observable UIA state.
The report is printed as JSON; the script exits non-zero on any failure.
#>
param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [int]$TimeoutSeconds = 30
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

$Automation = [System.Windows.Automation.AutomationElement]
$Scope = [System.Windows.Automation.TreeScope]
$ControlType = [System.Windows.Automation.ControlType]
$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
$report = [ordered]@{
    schema_version = 1
    kind = 'windows_ui_automation_smoke'
    executable_sha256 = (Get-FileHash -Algorithm SHA256 $Executable).Hash.ToLowerInvariant()
    checks = [ordered]@{}
    passed = $false
}

function Wait-For([scriptblock]$Condition, [string]$Description) {
    while ((Get-Date) -lt $deadline) {
        $value = & $Condition
        if ($value) { return $value }
        Start-Sleep -Milliseconds 100
    }
    throw "timed out waiting for $Description"
}

function Find-Named($Root, $Type, [string]$Name) {
    $condition = New-Object System.Windows.Automation.AndCondition(
        (New-Object System.Windows.Automation.PropertyCondition($Automation::ControlTypeProperty, $Type)),
        (New-Object System.Windows.Automation.PropertyCondition($Automation::NameProperty, $Name)))
    $Root.FindFirst($Scope::Descendants, $condition)
}

$process = Start-Process -FilePath $Executable -PassThru
try {
    $byProcess = New-Object System.Windows.Automation.PropertyCondition(
        $Automation::ProcessIdProperty, $process.Id)
    # winit also creates an unnamed message window for the same process.
    $window = Wait-For {
        $Automation::RootElement.FindAll($Scope::Children, $byProcess) |
            Where-Object { $_.Current.Name -eq 'Open UI accessibility' } |
            Select-Object -First 1
    } 'the application window'
    $report.checks.framework = $window.Current.FrameworkId
    if ($window.Current.FrameworkId -ne 'Open UI') { throw "unexpected framework $($window.Current.FrameworkId)" }

    $button = Wait-For { Find-Named $window $ControlType::Button 'Pressed 0 times' } 'the button'
    $checkbox = Wait-For { Find-Named $window $ControlType::CheckBox 'Remember choice' } 'the checkbox'
    $field = Wait-For { Find-Named $window $ControlType::Edit 'Name' } 'the text field'
    $report.checks.tree = 'button, checkbox and text field found by name and control type'

    $field.SetFocus()
    Wait-For { $field.Current.HasKeyboardFocus } 'text field focus' | Out-Null
    $button.SetFocus()
    Wait-For { $button.Current.HasKeyboardFocus } 'button focus' | Out-Null
    $report.checks.focus = 'focus moved to the text field and back to the button'

    $button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    Wait-For { Find-Named $window $ControlType::Button 'Pressed 1 times' } 'the first invoke' | Out-Null
    $button = Find-Named $window $ControlType::Button 'Pressed 1 times'
    $button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    Wait-For { Find-Named $window $ControlType::Button 'Pressed 2 times' } 'the second invoke' | Out-Null
    $report.checks.invoke = 'two invokes renamed the button to Pressed 2 times'

    $toggle = $checkbox.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)
    if ($toggle.Current.ToggleState -ne 'Off') { throw "checkbox starts $($toggle.Current.ToggleState)" }
    $toggle.Toggle()
    Wait-For { $toggle.Current.ToggleState -eq 'On' } 'the checkbox to turn on' | Out-Null
    $report.checks.toggle = 'checkbox toggled from Off to On'

    $value = $field.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
    $value.SetValue('Ada Lovelace')
    Wait-For { $value.Current.Value -eq 'Ada Lovelace' } 'the text field value' | Out-Null
    $report.checks.value = 'text field value set to Ada Lovelace'

    $window.GetCurrentPattern([System.Windows.Automation.WindowPattern]::Pattern).Close()
    if (-not $process.WaitForExit(10000)) { throw 'the window did not close' }
    $report.checks.close = "window closed; exit code $($process.ExitCode)"
    if ($process.ExitCode -ne 0) { throw "exit code $($process.ExitCode)" }
    $report.passed = $true
} catch {
    $report.error = "$_"
} finally {
    if (-not $process.HasExited) { $process.Kill() }
}
$report | ConvertTo-Json -Depth 4
if (-not $report.passed) { exit 1 }
