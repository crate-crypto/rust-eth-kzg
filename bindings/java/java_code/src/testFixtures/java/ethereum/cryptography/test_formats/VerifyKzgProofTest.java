package ethereum.cryptography.test_formats;

import com.fasterxml.jackson.annotation.JsonProperty;
import org.apache.tuweni.bytes.Bytes;

public class VerifyKzgProofTest {

  public static class Input {
    @JsonProperty("commitment")
    private String commitment;
    
    @JsonProperty("z")
    private String z;
    
    @JsonProperty("y")
    private String y;
    
    @JsonProperty("proof")
    private String proof;

    public byte[] getCommitment() {
      return Bytes.fromHexString(commitment).toArrayUnsafe();
    }

    public byte[] getZ() {
      return Bytes.fromHexString(z).toArrayUnsafe();
    }

    public byte[] getY() {
      return Bytes.fromHexString(y).toArrayUnsafe();
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