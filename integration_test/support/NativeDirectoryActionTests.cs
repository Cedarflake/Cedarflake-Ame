namespace Ame.Integration.Tests {
  using System;
  using System.Collections.Generic;

  public static class NativeDirectoryActionTests {
    private sealed class Dialog : INativeDirectoryDialog {
      public bool Exists = true;
      public bool Open = true;
      public bool ButtonReady = true;
      public bool IgnoreConfirm;
      public bool RejectConfirm;
      public bool IgnoreCancel;
      public bool ThrowOnClose;
      public int ReadinessDelay;
      public int CloseObservations;
      public readonly List<int> Clicks = new List<int>();
      public readonly Exception CloseFailure = new InvalidOperationException("close-observation-failed");
      private int pendingClose;

      public IntPtr FindDialog(int processId) {
        Check(processId == 41, "Dialog discovery escaped the owned process");
        return Exists ? new IntPtr(42) : IntPtr.Zero;
      }

      public IntPtr FindReadyButton(IntPtr dialog, int controlId) {
        Check(dialog == new IntPtr(42), "Button discovery changed the dialog owner");
        if (!ButtonReady || ReadinessDelay-- > 0) return IntPtr.Zero;
        return new IntPtr(controlId);
      }

      public bool IsOpen(IntPtr dialog, int processId) {
        Check(dialog == new IntPtr(42) && processId == 41, "Close observation changed ownership");
        if (Clicks.Count > 0 && ThrowOnClose) throw CloseFailure;
        if (pendingClose > 0) {
          CloseObservations++;
          if (--pendingClose == 0) Open = false;
        }
        return Open;
      }

      public bool Click(IntPtr button, int timeoutMilliseconds) {
        Check(timeoutMilliseconds > 0 && timeoutMilliseconds <= 500, "Native click is not bounded");
        var controlId = button.ToInt32();
        Clicks.Add(controlId);
        if (controlId == 1 && RejectConfirm) return false;
        if ((controlId == 1 && !IgnoreConfirm) || (controlId == 2 && !IgnoreCancel)) {
          pendingClose = 3;
        }
        return true;
      }
    }

    private static void Check(bool condition, string message) {
      if (!condition) throw new InvalidOperationException(message);
    }

    public static void Run() {
      foreach (var controlId in new[] { 1, 2 }) {
        var dialog = new Dialog { ReadinessDelay = 2 };
        var result = NativeDirectoryAction.Run(dialog, 41, controlId, 2000, 1000);
        Check(result.Succeeded && result.Stage == "closed", "A completed action failed");
        Check(dialog.CloseObservations == 3, "A click passed before dialog retirement");
        Check(dialog.Clicks.Count == 1 && dialog.Clicks[0] == controlId, "The requested action was repeated or changed");
      }

      var ignored = new Dialog { IgnoreConfirm = true };
      var ignoredResult = NativeDirectoryAction.Run(ignored, 41, 1, 100, 1000);
      Check(!ignoredResult.Succeeded && ignoredResult.Stage == "dialog_not_closed", "An ignored confirmation passed");
      Check(ignoredResult.Cleanup == "closed" && !ignored.Open, "A failed confirmation leaked its dialog");
      Check(ignored.Clicks.Count == 2 && ignored.Clicks[0] == 1 && ignored.Clicks[1] == 2, "Cleanup did not cancel the same dialog");

      var rejected = new Dialog { RejectConfirm = true, IgnoreCancel = true };
      var rejectedResult = NativeDirectoryAction.Run(rejected, 41, 1, 100, 100);
      Check(!rejectedResult.Succeeded && rejectedResult.Stage == "click_not_delivered", "Cleanup replaced the original click failure");
      Check(rejectedResult.Cleanup == "dialog_still_open", "An unclosed dialog was reported as cleaned up");

      var missing = new Dialog { Exists = false };
      var missingResult = NativeDirectoryAction.Run(missing, 41, 1, 100, 100);
      Check(!missingResult.Succeeded && missingResult.Stage == "dialog_not_found", "Missing-dialog diagnosis was lost");
      Check(missing.Clicks.Count == 0 && missingResult.Cleanup == "not_needed", "Cleanup acquired an unobserved dialog");

      var disabled = new Dialog { ButtonReady = false };
      var disabledResult = NativeDirectoryAction.Run(disabled, 41, 1, 100, 100);
      Check(!disabledResult.Succeeded && disabledResult.Stage == "button_not_ready", "An unready button passed");
      Check(disabled.Clicks.Count == 0, "An unready button was clicked");

      var disappeared = new Dialog { Open = false };
      var disappearedResult = NativeDirectoryAction.Run(disappeared, 41, 1, 100, 100);
      Check(!disappearedResult.Succeeded && disappearedResult.Stage == "closed_before_action", "An unrelated close passed as the requested action");

      var throwing = new Dialog { ThrowOnClose = true };
      Exception observed = null;
      try {
        NativeDirectoryAction.Run(throwing, 41, 1, 1000, 100);
      } catch (Exception error) {
        observed = error;
      }
      Check(object.ReferenceEquals(observed, throwing.CloseFailure), "Cleanup masked the native observation failure");
    }
  }
}
