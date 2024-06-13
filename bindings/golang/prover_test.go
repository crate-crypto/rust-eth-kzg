package peerdas_kzg

import (
	"testing"
)

func TestBridgeNewProverCtx(t *testing.T) {

	blob := make([]byte, 4096*32)
	blob[1] = 1
	prover_ctx := NewProverContext()
	comm, err := prover_ctx.BlobToKZGCommitment(blob)
	_ = comm
	_ = err
}
