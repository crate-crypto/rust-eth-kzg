package ethereum.cryptography.test_formats;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Arrays;
import org.apache.tuweni.bytes.Bytes;

public class VerifyBlobKzgProofBatchTest {

  public static class Input {
    @JsonProperty("blobs")
    private String[] blobs;
    
    @JsonProperty("commitments")
    private String[] commitments;
    
    @JsonProperty("proofs")
    private String[] proofs;

    public byte[][] getBlobs() {
      return Arrays.stream(blobs)
          .map(hexString -> Bytes.fromHexString(hexString).toArrayUnsafe())
          .toArray(byte[][]::new);
    }

    public byte[][] getCommitments() {
      return Arrays.stream(commitments)
          .map(hexString -> Bytes.fromHexString(hexString).toArrayUnsafe())
          .toArray(byte[][]::new);
    }

    public byte[][] getProofs() {
      return Arrays.stream(proofs)
          .map(hexString -> Bytes.fromHexString(hexString).toArrayUnsafe())
          .toArray(byte[][]::new);
    }
  }

  @JsonProperty("input")
  private Input input;

  @JsonProperty("output")
  private Boolean output;

  public Input getInput() {
    return input;
  }

  public Boolean getOutput() {
    return output;
  }
}