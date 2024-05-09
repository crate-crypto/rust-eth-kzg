package main

/*
#cgo LDFLAGS: ./lib/libc_peerdas_kzg.a -ldl
#include "./lib/c_peerdas_kzg.h"
*/
import "C"

func main() {

	prover_ctx := C.prover_context_new()
	out := make([]byte, 48) // Assume the output size is known to be 48 bytes; adjust as necessary.
	blob := make([]byte, 4096*32)
	C.blob_to_kzg_commitment(prover_ctx, (*C.uint8_t)(&blob[0]), (*C.uint8_t)(&out[0]))
}
