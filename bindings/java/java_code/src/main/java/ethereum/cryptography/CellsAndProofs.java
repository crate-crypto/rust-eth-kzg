package ethereum.cryptography;
import java.nio.ByteBuffer;
import java.util.Arrays;

public class CellsAndProofs {
    public byte[][] cells;
    public byte[][] proofs;

    public CellsAndProofs(byte[][] cells, byte[][] proofs) {
        this.cells = cells;
        this.proofs = proofs;
    }


  public byte[][] getCells() {
      return cells;
  }

  public byte[][] getProofs() {
      return proofs;
  }

  public static CellsAndProofs of(final byte[][] cells, final byte[][] proofs) {
      return new CellsAndProofs(cells, proofs);
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
      CellsAndProofs other = (CellsAndProofs) obj;
      return Arrays.deepEquals(cells, other.cells) && Arrays.deepEquals(proofs, other.proofs);
  }
}