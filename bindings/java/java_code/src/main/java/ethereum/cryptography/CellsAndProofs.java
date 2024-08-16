package ethereum.cryptography;

import java.nio.ByteBuffer;
import java.util.Arrays;

/**
 * Represents a pair of cells and their corresponding proofs in KZG
 * cryptography.
 */
public class CellsAndProofs {

    /** The array of cells. */
    public byte[][] cells;

    /** The array of proofs corresponding to the cells. */
    public byte[][] proofs;

    /**
     * Constructs a CellsAndProofs object with the given cells and proofs.
     *
     * @param cells  The array of cells.
     * @param proofs The array of proofs corresponding to the cells.
     */
    public CellsAndProofs(byte[][] cells, byte[][] proofs) {
        this.cells = cells;
        this.proofs = proofs;
    }

    /**
     * Gets the array of cells.
     *
     * @return The array of cells.
     */
    public byte[][] getCells() {
        return cells;
    }

    /**
     * Gets the array of proofs.
     *
     * @return The array of proofs.
     */
    public byte[][] getProofs() {
        return proofs;
    }

    /**
     * Creates a new CellsAndProofs object with the given cells and proofs.
     *
     * @param cells  The array of cells.
     * @param proofs The array of proofs corresponding to the cells.
     * @return A new CellsAndProofs object.
     */
    public static CellsAndProofs of(final byte[][] cells, final byte[][] proofs) {
        return new CellsAndProofs(cells, proofs);
    }

    /**
     * Converts the cells and proofs to a single byte array.
     *
     * @return A byte array containing all cells followed by all proofs.
     */
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