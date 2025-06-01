package ethereum.cryptography.test_formats;

import com.fasterxml.jackson.annotation.JsonProperty;
import org.apache.tuweni.bytes.Bytes;

public class VerifyBlobKzgProofTest {

  public static class Input {
    @JsonProperty("blob")
    private String blob;
    
    @JsonProperty("commitment")
    private String commitment;
    
    @JsonProperty("proof")
    private String proof;

    public byte[] getBlob() {
      return Bytes.fromHexString(blob).toArrayUnsafe();
    }

    public byte[] getCommitment() {
      return Bytes.fromHexString(commitment).toArrayUnsafe();
    }

    public byte[] getProof() {
      return Bytes.fromHexString(proof).toArrayUnsafe();
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