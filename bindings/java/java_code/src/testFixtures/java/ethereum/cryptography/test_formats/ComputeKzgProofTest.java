package ethereum.cryptography.test_formats;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Arrays;
import org.apache.tuweni.bytes.Bytes;

public class ComputeKzgProofTest {

  public static class Input {
    @JsonProperty("blob")
    private String blob;
    
    @JsonProperty("z")
    private String z;

    public byte[] getBlob() {
      return Bytes.fromHexString(blob).toArrayUnsafe();
    }

    public byte[] getZ() {
      return Bytes.fromHexString(z).toArrayUnsafe();
    }
  }

  @JsonProperty("input")
  private Input input;

  @JsonProperty("output")
  private String[] output;

  public Input getInput() {
    return input;
  }

  public byte[][] getOutput() {
    if (output == null) {
      return null;
    }
    return Arrays.stream(output)
        .map(hexString -> Bytes.fromHexString(hexString).toArrayUnsafe())
        .toArray(byte[][]::new);
  }
}