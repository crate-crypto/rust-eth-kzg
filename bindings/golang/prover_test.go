package peerdas_kzg

import (
	"testing"
)

func TestBridgeNewProverCtx(t *testing.T) {

	blob := make([]byte, 4096*32)
	blob[1] = 1
	prover_ctx := NewProverContext()
	cells, proofs := prover_ctx.ComputeCellsAndKZGProofs(blob)
	_ = cells
	_ = proofs
}
