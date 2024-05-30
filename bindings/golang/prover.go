package peerdas_kzg

/*
#cgo darwin,amd64 LDFLAGS: ./build/x86_64-apple-darwin/libc_peerdas_kzg.a
#cgo darwin,arm64 LDFLAGS: ./build/aarch64-apple-darwin/libc_peerdas_kzg.a
#cgo linux,amd64 LDFLAGS: ./build/x86_64-unknown-linux-gnu/libc_peerdas_kzg.a
#cgo linux,arm64 LDFLAGS: ./build/aarch64-unknown-linux-gnu/libc_peerdas_kzg.a
#cgo windows LDFLAGS: ./build/x86_64-pc-windows-gnu/libc_peerdas_kzg.a -lws2_32 -lntdll -luserenv
#include "./build/c_peerdas_kzg.h"
*/
import "C"
import "runtime"

type ProverContext struct {
	_inner *C.ProverContext
}

func NewProverContext() *ProverContext {
	self := &ProverContext{_inner: C.prover_context_new()}

	runtime.SetFinalizer(self, func(self *ProverContext) {
		C.prover_context_free(self.inner())
	})

	return &ProverContext{_inner: C.prover_context_new()}
}

func (prover *ProverContext) BlobToKZGCommitment(blob []byte) []byte {
	// TODO: We should add a check that the blob length is also correct by using a C constant
	// TODO: Take 48 from the C code constant
	out := make([]byte, 48)
	C.blob_to_kzg_commitment(prover.inner(), (*C.uint8_t)(&blob[0]), (*C.uint8_t)(&out[0]))
	return out
}

func (prover *ProverContext) ComputeCellsAndKZGProofs(blob []byte) ([]byte, []byte) {
	out_cells := make([]byte, C.NUM_BYTES_CELLS)
	out_proofs := make([]byte, C.NUM_BYTES_PROOFS)
	C.compute_cells_and_kzg_proofs(prover.inner(), (*C.uint8_t)(&blob[0]), (*C.uint8_t)(&out_cells[0]), (*C.uint8_t)(&out_proofs[0]))
	return out_cells, out_proofs
}

func (prover *ProverContext) inner() *C.ProverContext {
	return prover._inner
}
