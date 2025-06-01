package ethereum.cryptography.test_formats;

import com.fasterxml.jackson.annotation.JsonProperty;
import org.apache.tuweni.bytes.Bytes;

public class ComputeBlobKzgProofTest {

  public static class Input {
    @JsonProperty("blob")
    private String blob;
    
    @JsonProperty("commitment")
    private String commitment;

    public byte[] getBlob() {
      return Bytes.fromHexString(blob).toArrayUnsafe();
    }

    public byte[] getCommitment() {
      return Bytes.fromHexString(commitment).toArrayUnsafe();
    }
  }

  @JsonProperty("input")
  private Input input;

  @JsonProperty("output")
  private String output;

  public Input getInput() {
    return input;
  }

  public byte[] getOutput() {
    if (output == null) {
      return null;
    }
    return Bytes.fromHexString(output).toArrayUnsafe();
  }
}