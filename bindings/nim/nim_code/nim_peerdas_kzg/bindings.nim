import os
import build_utils

import header
export header

when defined(windows):
  # For gnu toolchain, the extension is .a since it uses the linux toolchain
  # This will need to be changed if we switch to the msvc toolchain
  const libName = "c_peerdas_kzg.lib"
else:
  const libName = "libc_peerdas_kzg.a"

const libpath = getInstallDir() / libName

{.passL: libpath.}

when defined(windows):
  {.passL: "-lws2_32".}
  {.passL: "-lntdll".}
  {.passL: "-luserenv".}
else:
  # Link math library for non-Windows platforms
  {.passL: "-lm".}