import os
import ../utils
import ../header
export header

when defined(windows):
  const libName = "c_peerdas_kzg.lib"
else:
  const libName = "libc_peerdas_kzg.a"

const libpath = getInstallDir() / libName

{.passL: libpath.}

proc add_from_rust(a: cint, b: cint): cint {.importc: "add123456789".}

# type
#   ProverContext* = pointer

# proc prover_context_new*(): ProverContext {.importc: "prover_context_new", cdecl.}

proc add_from_rust_wrapper*(a: int, b: int): int =
  ## Adds two numbers together.
  return add_from_rust(cint(a), cint(b))