import unittest

import nim_peerdas_kzg

test "prover context new and free":
  let context = prover_context_new()
  check context != nil
  prover_context_free(context)

test "verifier context new and free":
  let context = verifier_context_new()
  check context != nil
  verifier_context_free(context)