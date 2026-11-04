using System;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

namespace Ame.Integration {
  public interface INativeDirectoryDialog {
    IntPtr FindDialog(int processId);
    IntPtr FindReadyButton(IntPtr dialog, int controlId);
    bool IsOpen(IntPtr dialog, int processId);
    bool Click(IntPtr button, int timeoutMilliseconds);
  }

  public sealed class NativeDirectoryActionResult {
    public bool Succeeded { get; internal set; }
    public string Stage { get; internal set; }
    public long ElapsedMilliseconds { get; internal set; }
    public string Cleanup { get; internal set; }
  }

  public static class NativeDirectoryAction {
    public static NativeDirectoryActionResult Run(
      INativeDirectoryDialog native,
      int processId,
      int controlId,
      int timeoutMilliseconds,
      int cleanupMilliseconds
    ) {
      if (processId <= 0 || (controlId != 1 && controlId != 2) ||
          timeoutMilliseconds <= 0 || cleanupMilliseconds <= 0) {
        throw new ArgumentOutOfRangeException("Invalid native directory action arguments");
      }
      var clock = Stopwatch.StartNew();
      var dialog = IntPtr.Zero;
      var clicked = false;
      var result = new NativeDirectoryActionResult {
        Stage = "dialog_not_found", Cleanup = "not_needed"
      };
      try {
        while (clock.ElapsedMilliseconds < timeoutMilliseconds) {
          if (dialog == IntPtr.Zero) {
            dialog = native.FindDialog(processId);
          }
          if (dialog != IntPtr.Zero) {
            if (!native.IsOpen(dialog, processId)) {
              result.Succeeded = clicked;
              result.Stage = clicked ? "closed" : "closed_before_action";
              break;
            }
            if (!clicked) {
              result.Stage = "button_not_ready";
              var button = native.FindReadyButton(dialog, controlId);
              if (button != IntPtr.Zero) {
                var remaining = timeoutMilliseconds - (int)clock.ElapsedMilliseconds;
                if (remaining <= 0) break;
                if (!native.Click(button, Math.Min(remaining, 500))) {
                  result.Stage = "click_not_delivered";
                  break;
                }
                clicked = true;
                result.Stage = "dialog_not_closed";
              }
            }
          }
          Thread.Sleep(25);
        }
      } finally {
        result.ElapsedMilliseconds = clock.ElapsedMilliseconds;
        if (!result.Succeeded && dialog != IntPtr.Zero) {
          // Only the observed dialog in the requested process belongs to this action.
          result.Cleanup = Cancel(native, dialog, processId, cleanupMilliseconds);
        }
      }
      return result;
    }

    private static string Cancel(
      INativeDirectoryDialog native, IntPtr dialog, int processId, int timeoutMilliseconds
    ) {
      try {
        var clock = Stopwatch.StartNew();
        var clicked = false;
        while (clock.ElapsedMilliseconds < timeoutMilliseconds) {
          if (!native.IsOpen(dialog, processId)) return "closed";
          if (!clicked) {
            var cancel = native.FindReadyButton(dialog, 2);
            if (cancel != IntPtr.Zero) {
              var remaining = timeoutMilliseconds - (int)clock.ElapsedMilliseconds;
              if (remaining <= 0) break;
              if (!native.Click(cancel, Math.Min(remaining, 500))) return "click_failed";
              clicked = true;
            }
          }
          Thread.Sleep(25);
        }
        return "dialog_still_open";
      } catch (Exception error) {
        return "failed:" + error.GetType().Name;
      }
    }
  }

  public sealed class Win32DirectoryDialog : INativeDirectoryDialog {
    private delegate bool EnumWindowsProc(IntPtr window, IntPtr state);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool EnumWindows(EnumWindowsProc callback, IntPtr state);
    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll")]
    private static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")]
    private static extern bool IsWindow(IntPtr window);
    [DllImport("user32.dll")]
    private static extern bool IsWindowEnabled(IntPtr window);
    [DllImport("user32.dll")]
    private static extern bool EnumChildWindows(IntPtr parent, EnumWindowsProc callback, IntPtr state);
    [DllImport("user32.dll")]
    private static extern int GetDlgCtrlID(IntPtr window);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(IntPtr window, StringBuilder className, int capacity);
    [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern IntPtr SendMessageTimeout(
      IntPtr window, uint message, IntPtr wParam, IntPtr lParam,
      uint flags, uint timeout, out IntPtr result
    );

    private static bool HasClass(IntPtr window, string expected) {
      var buffer = new StringBuilder(256);
      GetClassName(window, buffer, buffer.Capacity);
      return string.Equals(buffer.ToString(), expected, StringComparison.Ordinal);
    }

    public bool IsOpen(IntPtr dialog, int processId) {
      uint owner;
      GetWindowThreadProcessId(dialog, out owner);
      return owner == (uint)processId && IsWindow(dialog) && HasClass(dialog, "#32770");
    }

    public IntPtr FindDialog(int processId) {
      var result = IntPtr.Zero;
      var matches = 0;
      var enumerated = EnumWindows((window, state) => {
        if (IsWindowVisible(window) && IsOpen(window, processId)) {
          result = window;
          matches++;
        }
        return true;
      }, IntPtr.Zero);
      if (!enumerated) throw new Win32Exception(Marshal.GetLastWin32Error());
      if (matches > 1) {
        throw new InvalidOperationException("Multiple native dialogs belong to the target process");
      }
      return result;
    }

    public IntPtr FindReadyButton(IntPtr dialog, int controlId) {
      var result = IntPtr.Zero;
      EnumChildWindows(dialog, (window, state) => {
        if (GetDlgCtrlID(window) == controlId && HasClass(window, "Button") &&
            IsWindowVisible(window) && IsWindowEnabled(window)) {
          result = window;
          return false;
        }
        return true;
      }, IntPtr.Zero);
      return result;
    }

    public bool Click(IntPtr button, int timeoutMilliseconds) {
      IntPtr result;
      // BM_CLICK is bounded even if the target stops dispatching native messages.
      return SendMessageTimeout(button, 0x00F5, IntPtr.Zero, IntPtr.Zero,
        0x0003, (uint)timeoutMilliseconds, out result) != IntPtr.Zero;
    }
  }
}
