using Microsoft.Extensions.FileSystemGlobbing;
using NUnit.Framework;
using YamlDotNet.Serialization;
using YamlDotNet.Serialization.NamingConventions;


// Testing code below taken from CKZG and modified to work with PeerDASKZG
namespace PeerDASKZG.test;

[TestFixture]
public class ReferenceTests
{
    [OneTimeSetUp]
    public void Setup()
    {

        _prover_context = PeerDASKZG.ProverContextNew();
        _deserializer = new DeserializerBuilder().WithNamingConvention(CamelCaseNamingConvention.Instance).Build();
    }

    [OneTimeTearDown]
    public void Teardown()
    {
        PeerDASKZG.ProverContextFree(_prover_context);
    }

    [TestCase]
    public void TestContextLoaded()
    {
        Assert.That(_prover_context, Is.Not.EqualTo(IntPtr.Zero));
    }

    private IntPtr _prover_context;
    private const string TestDir = "../../../../../../consensus_test_vectors";
    private readonly string _blobToKzgCommitmentTests = Path.Join(TestDir, "blob_to_kzg_commitment");
    private IDeserializer _deserializer;

    #region Helper Functions

    private static byte[] GetBytes(string hex)
    {
        return Convert.FromHexString(hex[2..]);
    }

    private static byte[] GetFlatBytes(List<string> strings)
    {
        List<byte[]> stringBytes = strings.Select(GetBytes).ToList();

        byte[] flatBytes = new byte[stringBytes.Sum(b => b.Length)];
        int offset = 0;
        foreach (byte[] bytes in stringBytes)
        {
            Buffer.BlockCopy(bytes, 0, flatBytes, offset, bytes.Length);
            offset += bytes.Length;
        }

        return flatBytes;
    }

    #endregion

    #region BlobToKzgCommitment

    private class BlobToKzgCommitmentInput
    {
        public string Blob { get; set; } = null!;
    }

    private class BlobToKzgCommitmentTest
    {
        public BlobToKzgCommitmentInput Input { get; set; } = null!;
        public string? Output { get; set; } = null!;
    }

    [TestCase]
    public void TestBlobToKzgCommitment()
    {
        Matcher matcher = new();
        matcher.AddIncludePatterns(new[] { "*/*/data.yaml" });

        IEnumerable<string> testFiles = matcher.GetResultsInFullPath(_blobToKzgCommitmentTests);
        Assert.That(testFiles.Count(), Is.GreaterThan(0));

        foreach (string testFile in testFiles)
        {

            string yaml = File.ReadAllText(testFile);
            BlobToKzgCommitmentTest test = _deserializer.Deserialize<BlobToKzgCommitmentTest>(yaml);
            Assert.That(test, Is.Not.EqualTo(null));

            byte[] commitment;
            byte[] blob = GetBytes(test.Input.Blob);

            try
            {

                commitment = PeerDASKZG.BlobToKzgCommitment(blob, _prover_context);
                Assert.That(test.Output, Is.Not.EqualTo(null), "blob length: ", blob.Length + "commitment: " + BitConverter.ToString(commitment) + " testFile: " + testFile);
                byte[] expectedCommitment = GetBytes(test.Output);
                Assert.That(commitment, Is.EqualTo(expectedCommitment));
            }
            catch
            {
                Assert.That(test.Output, Is.EqualTo(null));
            }
        }
    }

    #endregion
}