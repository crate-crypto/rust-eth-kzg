namespace PeerDASKZG;

public static partial class PeerDASKZG
{

    public static IntPtr ProverContextNew()
    {
        return InternalProverContextNew();
    }

    public static void ProverContextFree(IntPtr proverContextPtr)
    {
        InternalProverContextFree(proverContextPtr);
    }

    public static unsafe byte[] BlobToKzgCommitment(byte[] blob, IntPtr proverContextPtr)
    {
        // Sanity check input
        ThrowOnInvalidLength(blob, nameof(blob), 4096 * 32);

        byte[] commitment = new byte[48];
        Result result = InternalBlobToKzgCommitment(proverContextPtr, blob, commitment);
        ThrowOnError(result);
        return commitment;
    }

    // Below error handling was copied from ckzg
    private static void ThrowOnError(Result result)
    {
        switch (result)
        {
            case Result.Err: throw new ArgumentException("an error occurred from the bindings");
            case Result.Ok:
                return;
            default:
                throw new ApplicationException("PeerDASKZG returned an unexpected result variant");
        }
    }

    private static void ThrowOnInvalidLength(ReadOnlySpan<byte> data, string fieldName, int expectedLength)
    {
        if (data.Length != expectedLength)
            throw new ArgumentException("Invalid data size", fieldName);
    }
}