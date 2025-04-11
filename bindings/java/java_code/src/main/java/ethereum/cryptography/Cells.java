package ethereum.cryptography;

import java.nio.ByteBuffer;
import java.util.Arrays;

/**
 * Represents an array of cells in KZG cryptography.
 */
public class Cells {
    /** The array of cells. */
    public byte[][] cells;

    /**
     * Constructs a Cells object with the given cells.
     *
     * @param cells The array of cells.
     */
    public Cells(byte[][] cells) {
        this.cells = cells;
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
     * Creates a new Cells object with the given cells.
     *
     * @param cells The array of cells.
     * @return A new Cells object.
     */
    public static Cells of(final byte[][] cells) {
        return new Cells(cells);
    }

    /**
     * Converts the cells to a single byte array.
     *
     * @return A byte array containing all cells.
     */
    public byte[] toBytes() {
        int cellsLength = Arrays.stream(cells).mapToInt(cell -> cell.length).sum();
        ByteBuffer buffer = ByteBuffer.allocate(cellsLength);
        // Flatten cells
        Arrays.stream(cells).forEach(buffer::put);
        return buffer.array();
    }

    @Override
    public int hashCode() {
        final int prime = 31;
        int result = 1;
        result = prime * result + Arrays.deepHashCode(cells);
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
        Cells other = (Cells) obj;
        return Arrays.deepEquals(cells, other.cells);
    }
}