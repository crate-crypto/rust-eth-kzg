import nim_peerdas_kzg/bindings
export bindings

proc add2*(x, y: int): int =
  ## Adds two numbers together.
  return x + y
