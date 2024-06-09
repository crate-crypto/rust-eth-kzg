package ethereum.cryptography;
import java.nio.ByteBuffer;
import java.util.Arrays;

// This class is needed while we deprecate CellsAndProofs which uses a flattened
// version of cells and proofs

public class CellsAndProofsDeflattened {
    public byte[][] cells;
    public byte[][] proofs;

    public CellsAndProofsDeflattened(byte[][] cells, byte[][] proofs) {
        this.cells = cells;
        this.proofs = proofs;
    }


  public byte[][] getCells() {
      return cells;
  }

  public byte[][] getProofs() {
      return proofs;
  }

  public static CellsAndProofsDeflattened of(final byte[][] cells, final byte[][] proofs) {
      return new CellsAndProofsDeflattened(cells, proofs);
  }

  public byte[] toBytes() {
      int cellsLength = Arrays.stream(cells).mapToInt(cell -> cell.length).sum();
      int proofsLength = Arrays.stream(proofs).mapToInt(proof -> proof.length).sum();
      int totalLength = cellsLength + proofsLength;

      ByteBuffer buffer = ByteBuffer.allocate(totalLength);

      // Flatten cells
      Arrays.stream(cells).forEach(buffer::put);

      // Flatten proofs
      Arrays.stream(proofs).forEach(buffer::put);

      return buffer.array();
  }

  @Override
  public int hashCode() {
    int prime = 31;
    int result = 1;

    result = prime * result + Arrays.deepHashCode(cells);
    result = prime * result + Arrays.deepHashCode(proofs);

    return result;
  }

  @Override
  public boolean equals(Object obj) {
      if (this == obj) {
          return true;
      }
      if (obj == null || getClass() != obj.getClass()) {
          return false;
      }
      CellsAndProofsDeflattened other = (CellsAndProofsDeflattened) obj;
      return Arrays.deepEquals(cells, other.cells) && Arrays.deepEquals(proofs, other.proofs);
  }
}