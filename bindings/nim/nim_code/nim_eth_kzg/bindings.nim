import os
import build_utils

import header
export header

when defined(windows):
  # For gnu toolchain, the extension is .a since it uses the linux toolchain
  # This will need to be changed if we switch to the msvc toolchain
  const libName = "libc_eth_kzg.a"
else:
  const libName = "libc_eth_kzg.a"

# Path to the top level directory of the nim project
# so we can reference the build directory
const projectDir = currentSourcePath().parentDir().parentDir()
const libpath = projectDir / getInstallDir() / libName


{.passL: libpath.}

when defined(windows):
  {.passL: "-lws2_32".}
  {.passL: "-lntdll".}
  {.passL: "-luserenv".}
else:
  # Link math library for non-Windows platforms
  {.passL: "-lm".}