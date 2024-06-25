import os

const
  buildDir = "build"
  universalAppleDarwin {.used.} = buildDir / "universal-apple-darwin"
  x86_64PcWindowsMsvc {.used.} = buildDir / "x86_64-pc-windows-msvc"
  x86_64UnknownLinuxGnu {.used.} = buildDir / "x86_64-unknown-linux-gnu"
  aarch64UnknownLinuxGnu {.used.} = buildDir / "aarch64-unknown-linux-gnu"

proc getInstallDir*(): string =
  when defined(macosx):
    when defined(aarch64) or defined(amd64):
      return universalAppleDarwin
    else:
      raise newException(ValueError, "Unsupported architecture on macOS")
  elif defined(windows):
    when defined(amd64):
      return x86_64PcWindowsMsvc
    else:
      raise newException(ValueError, "Unsupported architecture on Windows")
  elif defined(linux):
    when defined(amd64):
      return x86_64UnknownLinuxGnu
    elif defined(aarch64):
      return aarch64UnknownLinuxGnu
    else:
      raise newException(ValueError, "Unsupported architecture on Linux")
  else:
    raise newException(ValueError, "Unsupported operating system")
