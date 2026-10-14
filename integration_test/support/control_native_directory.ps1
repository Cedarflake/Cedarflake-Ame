param(
  [Parameter(Mandatory = $true)]
  [int]$TargetProcessId,
  [ValidateSet("Confirm", "Cancel")]
  [string]$Action = "Confirm"
)

$ErrorActionPreference = "Stop"

Add-Type @"
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;

public static class AmeNativeDirectoryPicker {
  private delegate bool EnumWindowsProc(IntPtr window, IntPtr state);

  [DllImport("user32.dll", SetLastError = true)]
  private static extern bool EnumWindows(EnumWindowsProc callback, IntPtr state);

  [DllImport("user32.dll")]
  private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

  [DllImport("user32.dll")]
  private static extern bool IsWindowVisible(IntPtr window);

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

  public static IntPtr FindDialog(int processId) {
    var result = IntPtr.Zero;
    var matches = 0;
    var enumerated = EnumWindows((window, state) => {
      uint owner;
      GetWindowThreadProcessId(window, out owner);
      if (owner != (uint)processId || !IsWindowVisible(window)) {
        return true;
      }
      var buffer = new StringBuilder(256);
      GetClassName(window, buffer, buffer.Capacity);
      if (string.Equals(buffer.ToString(), "#32770", StringComparison.Ordinal)) {
        result = window;
        matches++;
      }
      return true;
    }, IntPtr.Zero);
    if (!enumerated) {
      throw new Win32Exception(Marshal.GetLastWin32Error());
    }
    if (matches > 1) {
      throw new InvalidOperationException("Multiple native dialogs belong to the target process");
    }
    return result;
  }

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

$dialogHandle = [IntPtr]::Zero
$controlId = if ($Action -eq "Confirm") { 1 } else { 2 }
$successMarker = if ($Action -eq "Confirm") {
  "NATIVE_PICKER_CONFIRMED"
} else {
  "NATIVE_PICKER_CANCELLED"
}

Write-Output "NATIVE_PICKER_READY"
$deadline = [DateTime]::UtcNow.AddSeconds(12)

do {
  # Inspect native dialog handles without activating Flutter's platform semantics owner.
  $dialogHandle = [AmeNativeDirectoryPicker]::FindDialog($TargetProcessId)
  if ($dialogHandle -ne [IntPtr]::Zero) {
    $buttonHandle = [AmeNativeDirectoryPicker]::FindDescendant(
      $dialogHandle,
      $controlId,
      "Button"
    )
    if ($buttonHandle -ne [IntPtr]::Zero) {
      [void][AmeNativeDirectoryPicker]::SendMessage(
        $buttonHandle,
        0x00F5,
        [IntPtr]::Zero,
        [IntPtr]::Zero
      )
      Write-Output $successMarker
      exit 0
    }
  }
  Start-Sleep -Milliseconds 100
} while ([DateTime]::UtcNow -lt $deadline)

if ($dialogHandle -ne [IntPtr]::Zero) {
  $cancelHandle = [AmeNativeDirectoryPicker]::FindDescendant(
    $dialogHandle,
    2,
    "Button"
  )
  if ($cancelHandle -ne [IntPtr]::Zero) {
    [void][AmeNativeDirectoryPicker]::SendMessage(
      $cancelHandle,
      0x00F5,
      [IntPtr]::Zero,
      [IntPtr]::Zero
    )
  }
}

Write-Error "Native directory picker $Action button was not found"
exit 2
