import ctypes
from ctypes import wintypes
import faulthandler
import json
import os
import pathlib
import sys


kernel = ctypes.WinDLL("kernel32", use_last_error=True)
user = ctypes.WinDLL("user32", use_last_error=True)
psapi = ctypes.WinDLL("psapi", use_last_error=True)
callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)


class ProcessEntry(ctypes.Structure):
  _fields_ = [
    ("size", wintypes.DWORD), ("usage", wintypes.DWORD),
    ("pid", wintypes.DWORD), ("heap", ctypes.c_size_t),
    ("module", wintypes.DWORD), ("threads", wintypes.DWORD),
    ("parent", wintypes.DWORD), ("priority", wintypes.LONG),
    ("flags", wintypes.DWORD), ("name", wintypes.WCHAR * 260),
  ]


class ProcessMemory(ctypes.Structure):
  _fields_ = [
    ("size", wintypes.DWORD), ("faults", wintypes.DWORD),
    ("peak_working_set", ctypes.c_size_t), ("working_set", ctypes.c_size_t),
    ("peak_paged", ctypes.c_size_t), ("paged", ctypes.c_size_t),
    ("peak_nonpaged", ctypes.c_size_t), ("nonpaged", ctypes.c_size_t),
    ("pagefile", ctypes.c_size_t), ("peak_pagefile", ctypes.c_size_t),
    ("private", ctypes.c_size_t),
  ]


kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
kernel.OpenProcess.restype = wintypes.HANDLE
kernel.CloseHandle.argtypes = [wintypes.HANDLE]
kernel.CloseHandle.restype = wintypes.BOOL
kernel.QueryFullProcessImageNameW.argtypes = [
  wintypes.HANDLE, wintypes.DWORD, wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD),
]
kernel.QueryFullProcessImageNameW.restype = wintypes.BOOL
kernel.CreateToolhelp32Snapshot.argtypes = [wintypes.DWORD, wintypes.DWORD]
kernel.CreateToolhelp32Snapshot.restype = wintypes.HANDLE
for method in (kernel.Process32FirstW, kernel.Process32NextW):
  method.argtypes = [wintypes.HANDLE, ctypes.POINTER(ProcessEntry)]
  method.restype = wintypes.BOOL
user.EnumWindows.argtypes = [callback_type, wintypes.LPARAM]
user.EnumWindows.restype = wintypes.BOOL
user.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
user.GetWindowThreadProcessId.restype = wintypes.DWORD
user.GetClassNameW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
user.GetClassNameW.restype = ctypes.c_int
user.SendMessageTimeoutW.argtypes = [
  wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM,
  wintypes.UINT, wintypes.UINT, ctypes.POINTER(ctypes.c_size_t),
]
user.SendMessageTimeoutW.restype = wintypes.LPARAM
psapi.GetProcessMemoryInfo.argtypes = [
  wintypes.HANDLE, ctypes.POINTER(ProcessMemory), wintypes.DWORD,
]
psapi.GetProcessMemoryInfo.restype = wintypes.BOOL


def process_path(handle):
  path = ctypes.create_unicode_buffer(32768)
  length = wintypes.DWORD(len(path))
  if not kernel.QueryFullProcessImageNameW(handle, 0, path, ctypes.byref(length)):
    raise ctypes.WinError(ctypes.get_last_error())
  return pathlib.Path(path.value).resolve(strict=True)


def validate_client(pid):
  if pid != os.getppid():
    raise ValueError("Only the invoking integration client may receive power events")
  handle = kernel.OpenProcess(0x1000, False, pid)
  if not handle:
    raise ctypes.WinError(ctypes.get_last_error())
  try:
    expected = pathlib.Path(__file__).resolve().parents[2] / "build/windows/x64/runner/Debug"
    if process_path(handle) != expected / "cedarflake_ame.exe":
      raise ValueError("Power verification requires the repository Debug client")
    return expected
  finally:
    kernel.CloseHandle(handle)


def power_event(pid, event):
  windows = []

  @callback_type
  def collect(window, _):
    owner = wintypes.DWORD()
    user.GetWindowThreadProcessId(window, ctypes.byref(owner))
    name = ctypes.create_unicode_buffer(128)
    user.GetClassNameW(window, name, len(name))
    if owner.value == pid and name.value == "FLUTTER_RUNNER_WIN32_WINDOW":
      windows.append(window)
    return True

  if not user.EnumWindows(collect, 0):
    raise ctypes.WinError(ctypes.get_last_error())
  if len(windows) != 1:
    raise ValueError("Exactly one owned runner window is required")
  result = ctypes.c_size_t()
  if not user.SendMessageTimeoutW(windows[0], 0x218, event, 0, 2, 5000, ctypes.byref(result)):
    raise ctypes.WinError(ctypes.get_last_error())
  if result.value != 1:
    raise ValueError("The production runner did not acknowledge its power event")
  return {"delivered": True}


def workers(pid, directory):
  snapshot = kernel.CreateToolhelp32Snapshot(2, 0)
  if snapshot == ctypes.c_void_p(-1).value:
    raise ctypes.WinError(ctypes.get_last_error())
  result = []
  try:
    entry = ProcessEntry(size=ctypes.sizeof(ProcessEntry))
    found = kernel.Process32FirstW(snapshot, ctypes.byref(entry))
    while found:
      if entry.parent == pid and entry.name == "cedarflake_ame_classification_worker.exe":
        handle = kernel.OpenProcess(0x1010, False, entry.pid)
        if handle:
          try:
            if process_path(handle) != directory / entry.name:
              raise ValueError("Unexpected classification worker executable")
            memory = ProcessMemory(size=ctypes.sizeof(ProcessMemory))
            if not psapi.GetProcessMemoryInfo(handle, ctypes.byref(memory), memory.size):
              raise ctypes.WinError(ctypes.get_last_error())
            result.append({
              "pid": entry.pid, "workingSetBytes": memory.working_set,
              "peakWorkingSetBytes": memory.peak_working_set, "privateBytes": memory.private,
            })
          finally:
            kernel.CloseHandle(handle)
        elif ctypes.get_last_error() != 87:
          raise ctypes.WinError(ctypes.get_last_error())
      found = kernel.Process32NextW(snapshot, ctypes.byref(entry))
    if ctypes.get_last_error() != 18:
      raise ctypes.WinError(ctypes.get_last_error())
  finally:
    kernel.CloseHandle(snapshot)
  return result


if __name__ == "__main__":
  faulthandler.dump_traceback_later(4)
  client = int(sys.argv[1])
  directory = validate_client(client)
  action = sys.argv[2]
  if action == "suspend":
    result = power_event(client, 4)
  elif action == "resume":
    result = power_event(client, 18)
  elif action == "workers":
    result = workers(client, directory)
  else:
    raise ValueError("Unknown classification power action")
  print(json.dumps(result, separators=(",", ":")))
  faulthandler.cancel_dump_traceback_later()
