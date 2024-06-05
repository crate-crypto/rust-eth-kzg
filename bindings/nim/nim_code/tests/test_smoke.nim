import unittest

import nim_peerdas_kzg

test "context new and free":
  let context = peerdas_context_new()
  check context != nil
  peerdas_context_free(context)
