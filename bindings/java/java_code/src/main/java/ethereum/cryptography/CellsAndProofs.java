package ethereum.cryptography;

public class CellsAndProofs {
    public byte[] cells;
    public byte[] proofs;

    public CellsAndProofs(byte[] cells, byte[] proofs) {
        this.cells = cells;
        this.proofs = proofs;
    }
}