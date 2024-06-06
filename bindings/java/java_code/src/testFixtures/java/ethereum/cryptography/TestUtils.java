package ethereum.cryptography;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.dataformat.yaml.YAMLFactory;
import ethereum.cryptography.test_formats.*;
import java.io.BufferedReader;
import java.io.FileReader;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.math.BigInteger;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Random;
import java.util.stream.Collectors;
import java.util.stream.IntStream;
import java.util.stream.Stream;
import org.apache.tuweni.bytes.Bytes;
import org.apache.tuweni.units.bigints.UInt256;

public class TestUtils {

  private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper(new YAMLFactory());

  private static final String BLOB_TO_KZG_COMMITMENT_TESTS = "../../../consensus_test_vectors/blob_to_kzg_commitment/";
  private static final String COMPUTE_CELLS_TESTS = "../../../consensus_test_vectors/compute_cells/";
  private static final String COMPUTE_CELLS_AND_KZG_PROOFS_TESTS =
      "..../consensus_test_vectors/compute_cells_and_kzg_proofs/";
  private static final String VERIFY_CELL_KZG_PROOF_TESTS = "../../../consensus_test_vectors/verify_cell_kzg_proof/";
  private static final String VERIFY_CELL_KZG_PROOF_BATCH_TESTS =
      "..../consensus_test_vectors/verify_cell_kzg_proof_batch/";
  private static final String RECOVER_ALL_CELLS_TESTS = "../../../consensus_test_vectors/recover_all_cells/";

  public static byte[] flatten(final byte[]... bytes) {
    final int capacity = Arrays.stream(bytes).mapToInt(b -> b.length).sum();
    final ByteBuffer buffer = ByteBuffer.allocate(capacity);
    Arrays.stream(bytes).forEach(buffer::put);
    return buffer.array();
  }

  public static List<BlobToKzgCommitmentTest> getBlobToKzgCommitmentTests() {
    final Stream.Builder<BlobToKzgCommitmentTest> tests = Stream.builder();
    List<String> testFiles = getTestFiles(BLOB_TO_KZG_COMMITMENT_TESTS);

    assert !testFiles.isEmpty();

    try {
      for (String testFile : testFiles) {
        String data = Files.readString(Path.of(testFile));
        BlobToKzgCommitmentTest test = OBJECT_MAPPER.readValue(data, BlobToKzgCommitmentTest.class);
        tests.add(test);
      }
    } catch (IOException ex) {
      throw new UncheckedIOException(ex);
    }

    return tests.build().collect(Collectors.toList());
  }

  public static List<ComputeCellsTest> getComputeCellsTests() {
    final Stream.Builder<ComputeCellsTest> tests = Stream.builder();
    List<String> testFiles = getTestFiles(COMPUTE_CELLS_TESTS);
    assert !testFiles.isEmpty();

    try {
      for (String testFile : testFiles) {
        String jsonData = Files.readString(Path.of(testFile));
        ComputeCellsTest test = OBJECT_MAPPER.readValue(jsonData, ComputeCellsTest.class);
        tests.add(test);
      }
    } catch (IOException ex) {
      throw new UncheckedIOException(ex);
    }

    return tests.build().collect(Collectors.toList());
  }

  public static List<ComputeCellsAndKzgProofsTest> getComputeCellsAndKzgProofsTests() {
    final Stream.Builder<ComputeCellsAndKzgProofsTest> tests = Stream.builder();
    List<String> testFiles = getTestFiles(COMPUTE_CELLS_AND_KZG_PROOFS_TESTS);
    assert !testFiles.isEmpty();

    try {
      for (String testFile : testFiles) {
        String jsonData = Files.readString(Path.of(testFile));
        ComputeCellsAndKzgProofsTest test =
            OBJECT_MAPPER.readValue(jsonData, ComputeCellsAndKzgProofsTest.class);
        tests.add(test);
      }
    } catch (IOException ex) {
      throw new UncheckedIOException(ex);
    }

    return tests.build().collect(Collectors.toList());
  }

  public static List<VerifyCellKzgProofTest> getVerifyCellKzgProofTests() {
    final Stream.Builder<VerifyCellKzgProofTest> tests = Stream.builder();
    List<String> testFiles = getTestFiles(VERIFY_CELL_KZG_PROOF_TESTS);
    assert !testFiles.isEmpty();

    try {
      for (String testFile : testFiles) {
        String jsonData = Files.readString(Path.of(testFile));
        VerifyCellKzgProofTest test =
            OBJECT_MAPPER.readValue(jsonData, VerifyCellKzgProofTest.class);
        tests.add(test);
      }
    } catch (IOException ex) {
      throw new UncheckedIOException(ex);
    }

    return tests.build().collect(Collectors.toList());
  }

  public static List<VerifyCellKzgProofBatchTest> getVerifyCellKzgProofBatchTests() {
    final Stream.Builder<VerifyCellKzgProofBatchTest> tests = Stream.builder();
    List<String> testFiles = getTestFiles(VERIFY_CELL_KZG_PROOF_BATCH_TESTS);
    assert !testFiles.isEmpty();

    try {
      for (String testFile : testFiles) {
        String jsonData = Files.readString(Path.of(testFile));
        VerifyCellKzgProofBatchTest test =
            OBJECT_MAPPER.readValue(jsonData, VerifyCellKzgProofBatchTest.class);
        tests.add(test);
      }
    } catch (IOException ex) {
      throw new UncheckedIOException(ex);
    }

    return tests.build().collect(Collectors.toList());
  }

  public static List<RecoverAllCellsTest> getRecoverAllCellsTests() {
    final Stream.Builder<RecoverAllCellsTest> tests = Stream.builder();
    List<String> testFiles = getTestFiles(RECOVER_ALL_CELLS_TESTS);
    assert !testFiles.isEmpty();

    try {
      for (String testFile : testFiles) {
        String jsonData = Files.readString(Path.of(testFile));
        RecoverAllCellsTest test = OBJECT_MAPPER.readValue(jsonData, RecoverAllCellsTest.class);
        tests.add(test);
      }
    } catch (IOException ex) {
      throw new UncheckedIOException(ex);
    }

    return tests.build().collect(Collectors.toList());
  }
  
  public static List<String> getFiles(String path) {
    try {
      try (Stream<Path> stream = Files.list(Paths.get(path))) {
        return stream.map(Path::toString).sorted().collect(Collectors.toList());
      }
    } catch (final IOException ex) {
      throw new UncheckedIOException(ex);
    }
  }

  public static List<String> getTestFiles(String path) {
    List<String> testFiles = new ArrayList<>();
    for (final String suite : getFiles(path)) {
      for (final String test : getFiles(suite)) {
        testFiles.addAll(getFiles(test));
      }
    }
    return testFiles;
  }
}
