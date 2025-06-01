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

  private static final String BLOB_TO_KZG_COMMITMENT_TESTS = "../../../test_vectors/blob_to_kzg_commitment/";
  private static final String COMPUTE_CELLS_AND_KZG_PROOFS_TESTS =
      "../../../test_vectors/compute_cells_and_kzg_proofs/";
  private static final String VERIFY_CELL_KZG_PROOF_BATCH_TESTS =
      "../../../test_vectors/verify_cell_kzg_proof_batch/";
  private static final String RECOVER_CELLS_AND_KZG_PROOFS_TESTS = "../../../test_vectors/recover_cells_and_kzg_proofs/";
  private static final String COMPUTE_KZG_PROOF_TESTS = "../../../test_vectors/compute_kzg_proof/";
  private static final String COMPUTE_BLOB_KZG_PROOF_TESTS = "../../../test_vectors/compute_blob_kzg_proof/";
  private static final String VERIFY_KZG_PROOF_TESTS = "../../../test_vectors/verify_kzg_proof/";
  private static final String VERIFY_BLOB_KZG_PROOF_TESTS = "../../../test_vectors/verify_blob_kzg_proof/";
  private static final String VERIFY_BLOB_KZG_PROOF_BATCH_TESTS = "../../../test_vectors/verify_blob_kzg_proof_batch/";

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

  public static List<RecoverCellsAndKzgProofsTest> getRecoverCellsAndKzgProofsTests() {
    final Stream.Builder<RecoverCellsAndKzgProofsTest> tests = Stream.builder();
    List<String> testFiles = getTestFiles(RECOVER_CELLS_AND_KZG_PROOFS_TESTS);
    assert !testFiles.isEmpty();

    try {
      for (String testFile : testFiles) {
        String jsonData = Files.readString(Path.of(testFile));
        RecoverCellsAndKzgProofsTest test =
            OBJECT_MAPPER.readValue(jsonData, RecoverCellsAndKzgProofsTest.class);
        tests.add(test);
      }
    } catch (IOException ex) {
      throw new UncheckedIOException(ex);
    }

    return tests.build().collect(Collectors.toList());
  }

  public static List<ComputeKzgProofTest> getComputeKzgProofTests() {
    final Stream.Builder<ComputeKzgProofTest> tests = Stream.builder();
    List<String> testFiles = getTestFiles(COMPUTE_KZG_PROOF_TESTS);
    assert !testFiles.isEmpty();

    try {
      for (String testFile : testFiles) {
        String jsonData = Files.readString(Path.of(testFile));
        ComputeKzgProofTest test =
            OBJECT_MAPPER.readValue(jsonData, ComputeKzgProofTest.class);
        tests.add(test);
      }
    } catch (IOException ex) {
      throw new UncheckedIOException(ex);
    }

    return tests.build().collect(Collectors.toList());
  }

  public static List<ComputeBlobKzgProofTest> getComputeBlobKzgProofTests() {
    final Stream.Builder<ComputeBlobKzgProofTest> tests = Stream.builder();
    List<String> testFiles = getTestFiles(COMPUTE_BLOB_KZG_PROOF_TESTS);
    assert !testFiles.isEmpty();

    try {
      for (String testFile : testFiles) {
        String jsonData = Files.readString(Path.of(testFile));
        ComputeBlobKzgProofTest test =
            OBJECT_MAPPER.readValue(jsonData, ComputeBlobKzgProofTest.class);
        tests.add(test);
      }
    } catch (IOException ex) {
      throw new UncheckedIOException(ex);
    }

    return tests.build().collect(Collectors.toList());
  }

  public static List<VerifyKzgProofTest> getVerifyKzgProofTests() {
    final Stream.Builder<VerifyKzgProofTest> tests = Stream.builder();
    List<String> testFiles = getTestFiles(VERIFY_KZG_PROOF_TESTS);
    assert !testFiles.isEmpty();

    try {
      for (String testFile : testFiles) {
        String jsonData = Files.readString(Path.of(testFile));
        VerifyKzgProofTest test =
            OBJECT_MAPPER.readValue(jsonData, VerifyKzgProofTest.class);
        tests.add(test);
      }
    } catch (IOException ex) {
      throw new UncheckedIOException(ex);
    }

    return tests.build().collect(Collectors.toList());
  }

  public static List<VerifyBlobKzgProofTest> getVerifyBlobKzgProofTests() {
    final Stream.Builder<VerifyBlobKzgProofTest> tests = Stream.builder();
    List<String> testFiles = getTestFiles(VERIFY_BLOB_KZG_PROOF_TESTS);
    assert !testFiles.isEmpty();

    try {
      for (String testFile : testFiles) {
        String jsonData = Files.readString(Path.of(testFile));
        VerifyBlobKzgProofTest test =
            OBJECT_MAPPER.readValue(jsonData, VerifyBlobKzgProofTest.class);
        tests.add(test);
      }
    } catch (IOException ex) {
      throw new UncheckedIOException(ex);
    }

    return tests.build().collect(Collectors.toList());
  }

  public static List<VerifyBlobKzgProofBatchTest> getVerifyBlobKzgProofBatchTests() {
    final Stream.Builder<VerifyBlobKzgProofBatchTest> tests = Stream.builder();
    List<String> testFiles = getTestFiles(VERIFY_BLOB_KZG_PROOF_BATCH_TESTS);
    assert !testFiles.isEmpty();

    try {
      for (String testFile : testFiles) {
        String jsonData = Files.readString(Path.of(testFile));
        VerifyBlobKzgProofBatchTest test =
            OBJECT_MAPPER.readValue(jsonData, VerifyBlobKzgProofBatchTest.class);
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
