param(
  [Parameter(Mandatory = $true)]
  [int]$TargetProcessId
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class AmeNativePicker {
  private delegate bool EnumWindowsProc(IntPtr window, IntPtr state);

  [DllImport("user32.dll")]
  private static extern bool EnumChildWindows(
    IntPtr parent,
    EnumWindowsProc callback,
    IntPtr state
  );

  [DllImport("user32.dll")]
  private static extern int GetDlgCtrlID(IntPtr window);

  [DllImport("user32.dll", CharSet = CharSet.Unicode)]
  private static extern int GetClassName(
    IntPtr window,
    StringBuilder className,
    int capacity
  );

  [DllImport("user32.dll")]
  public static extern IntPtr SendMessage(
    IntPtr window,
    uint message,
    IntPtr wParam,
    IntPtr lParam
  );

  public static IntPtr FindDescendant(
    IntPtr parent,
    int controlId,
    string className
  ) {
    var result = IntPtr.Zero;
    EnumChildWindows(parent, (window, state) => {
      var buffer = new StringBuilder(256);
      GetClassName(window, buffer, buffer.Capacity);
      if (
        GetDlgCtrlID(window) == controlId &&
        string.Equals(buffer.ToString(), className, StringComparison.Ordinal)
      ) {
        result = window;
        return false;
      }
      return true;
    }, IntPtr.Zero);
    return result;
  }
}
"@

$processCondition = New-Object System.Windows.Automation.PropertyCondition(
  [System.Windows.Automation.AutomationElement]::ProcessIdProperty,
  $TargetProcessId
)
$dialogCondition = New-Object System.Windows.Automation.PropertyCondition(
  [System.Windows.Automation.AutomationElement]::ClassNameProperty,
  "#32770"
)
$deadline = [DateTime]::UtcNow.AddSeconds(12)
$dialog = $null

do {
  $app = [System.Windows.Automation.AutomationElement]::RootElement.FindFirst(
    [System.Windows.Automation.TreeScope]::Children,
    $processCondition
  )
  if ($app) {
    $dialog = $app.FindFirst(
      [System.Windows.Automation.TreeScope]::Descendants,
      $dialogCondition
    )
    if ($dialog) {
      $dialogHandle = [IntPtr]$dialog.Current.NativeWindowHandle
      $buttonHandle = [AmeNativePicker]::FindDescendant(
        $dialogHandle,
        1,
        "Button"
      )
      if ($buttonHandle -ne [IntPtr]::Zero) {
        [void][AmeNativePicker]::SendMessage(
          $buttonHandle,
          0x00F5,
          [IntPtr]::Zero,
          [IntPtr]::Zero
        )
        Write-Output "NATIVE_PICKER_CONFIRMED"
        exit 0
      }
    }
  }
  Start-Sleep -Milliseconds 100
} while ([DateTime]::UtcNow -lt $deadline)

if ($dialog) {
  $dialogHandle = [IntPtr]$dialog.Current.NativeWindowHandle
  $cancelHandle = [AmeNativePicker]::FindDescendant(
    $dialogHandle,
    2,
    "Button"
  )
  if ($cancelHandle -ne [IntPtr]::Zero) {
    [void][AmeNativePicker]::SendMessage(
      $cancelHandle,
      0x00F5,
      [IntPtr]::Zero,
      [IntPtr]::Zero
    )
  }
}

Write-Error "Native directory picker confirmation button was not found"
exit 2
