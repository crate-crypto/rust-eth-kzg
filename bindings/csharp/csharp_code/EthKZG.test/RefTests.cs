using Microsoft.Extensions.FileSystemGlobbing;
using NUnit.Framework;
using YamlDotNet.Serialization;
using YamlDotNet.Serialization.NamingConventions;


// Testing code below taken from CKZG and modified to work with EthKZG
namespace EthKZG.test;

[TestFixture]
public class ReferenceTests
{
    [OneTimeSetUp]
    public void Setup()
    {

        _context = new EthKZG();
        _deserializer = new DeserializerBuilder().WithNamingConvention(CamelCaseNamingConvention.Instance).Build();
        // Note: On some systems, this is needed as the normal deserializer has trouble deserializing
        // `cell_id` to `CellId` ie the underscore is not being parsed correctly.
        _deserializerUnderscoreNaming = new DeserializerBuilder().WithNamingConvention(UnderscoredNamingConvention.Instance).Build();
    }

    [OneTimeTearDown]
    public void Teardown()
    {
        _context.Dispose();
    }


    private EthKZG _context;
    private const string TestDir = "../../../../../../../test_vectors";
    private readonly string _blobToKzgCommitmentTests = Path.Join(TestDir, "blob_to_kzg_commitment");
    private readonly string _computeCellsAndKzgProofsTests = Path.Join(TestDir, "compute_cells_and_kzg_proofs");
    private readonly string _verifyCellKzgProofBatchTests = Path.Join(TestDir, "verify_cell_kzg_proof_batch");
    private readonly string _recoverCellsAndKzgProofsTests = Path.Join(TestDir, "recover_cells_and_kzg_proofs");
    
    // EIP-4844 test directories
    private readonly string _computeKzgProofTests = Path.Join(TestDir, "compute_kzg_proof");
    private readonly string _computeBlobKzgProofTests = Path.Join(TestDir, "compute_blob_kzg_proof");
    private readonly string _verifyKzgProofTests = Path.Join(TestDir, "verify_kzg_proof");
    private readonly string _verifyBlobKzgProofTests = Path.Join(TestDir, "verify_blob_kzg_proof");
    private readonly string _verifyBlobKzgProofBatchTests = Path.Join(TestDir, "verify_blob_kzg_proof_batch");

    private IDeserializer _deserializer;
    private IDeserializer _deserializerUnderscoreNaming;

    #region Helper Functions

    private static byte[] GetBytes(string hex)
    {
        return Convert.FromHexString(hex[2..]);
    }

    private static byte[][] GetByteArrays(List<string> strings)
    {
        return strings.Select(GetBytes).ToArray();
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

                commitment = _context.BlobToKzgCommitment(blob);
                Assert.That(test.Output, Is.Not.EqualTo(null));
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

    #region ComputeCellsAndKzgProofs

    private class ComputeCellsAndKzgProofsInput
    {
        public string Blob { get; set; } = null!;
    }

    private class ComputeCellsAndKzgProofsTest
    {
        public ComputeCellsAndKzgProofsInput Input { get; set; } = null!;
        public List<List<string>>? Output { get; set; } = null!;
    }

    [TestCase]
    public void TestComputeCellsAndKzgProofs()
    {
        Matcher matcher = new();
        matcher.AddIncludePatterns(new[] { "*/*/data.yaml" });

        IEnumerable<string> testFiles = matcher.GetResultsInFullPath(_computeCellsAndKzgProofsTests);
        Assert.That(testFiles.Count(), Is.GreaterThan(0));

        foreach (string testFile in testFiles)
        {
            string yaml = File.ReadAllText(testFile);
            ComputeCellsAndKzgProofsTest test = _deserializer.Deserialize<ComputeCellsAndKzgProofsTest>(yaml);
            Assert.That(test, Is.Not.EqualTo(null));

            byte[] blob = GetBytes(test.Input.Blob);

            try
            {
                (byte[][] cells, byte[][] proofs) = _context.ComputeCellsAndKZGProofs(blob);
                Assert.That(test.Output, Is.Not.EqualTo(null));
                byte[][] expectedCells = GetByteArrays(test.Output.ElementAt(0));
                Assert.That(cells, Is.EqualTo(expectedCells));
                byte[][] expectedProofs = GetByteArrays(test.Output.ElementAt(1));
                Assert.That(proofs, Is.EqualTo(expectedProofs));

                byte[][] cells_ = _context.ComputeCells(blob);
                Assert.That(cells_, Is.EqualTo(expectedCells));
            }
            catch
            {
                Assert.That(test.Output, Is.EqualTo(null));
            }
        }
    }

    #endregion

    #region VerifyCellKzgProofBatch

    private class VerifyCellKzgProofBatchInput
    {
        public List<string> Commitments { get; set; } = null!;
        public List<ulong> CellIndices { get; set; } = null!;
        public List<string> Cells { get; set; } = null!;
        public List<string> Proofs { get; set; } = null!;
    }

    private class VerifyCellKzgProofBatchTest
    {
        public VerifyCellKzgProofBatchInput Input { get; set; } = null!;
        public bool? Output { get; set; } = null!;
    }

    [TestCase]
    public void TestVerifyCellKzgProofBatch()
    {
        Matcher matcher = new();
        matcher.AddIncludePatterns(new[] { "*/*/data.yaml" });

        IEnumerable<string> testFiles = matcher.GetResultsInFullPath(_verifyCellKzgProofBatchTests);
        Assert.That(testFiles.Count(), Is.GreaterThan(0));

        foreach (string testFile in testFiles)
        {
            string yaml = File.ReadAllText(testFile);
            VerifyCellKzgProofBatchTest test = _deserializerUnderscoreNaming.Deserialize<VerifyCellKzgProofBatchTest>(yaml);
            Assert.That(test, Is.Not.EqualTo(null));

            byte[][] commitments = GetByteArrays(test.Input.Commitments);
            ulong[] cellIndices = test.Input.CellIndices.ToArray();
            byte[][] cells = GetByteArrays(test.Input.Cells);
            byte[][] proofs = GetByteArrays(test.Input.Proofs);

            try
            {
                bool isCorrect = _context.VerifyCellKZGProofBatch(commitments, cellIndices, cells, proofs);
                Assert.That(isCorrect, Is.EqualTo(test.Output));
            }
            catch
            {
                Assert.That(test.Output, Is.EqualTo(null));
            }
        }
    }

    #endregion

    #region RecoverCellsAndKzgProofs

    private class RecoverCellsAndKzgProofsInput
    {
        public List<ulong> CellIndices { get; set; } = null!;
        public List<string> Cells { get; set; } = null!;
    }

    private class RecoverCellsAndKzgProofsTest
    {
        public RecoverCellsAndKzgProofsInput Input { get; set; } = null!;
        public List<List<string>>? Output { get; set; } = null!;
    }

    [TestCase]
    public void TestRecoverCellsAndKzgProofs()
    {
        Matcher matcher = new();
        matcher.AddIncludePatterns(new[] { "*/*/data.yaml" });

        IEnumerable<string> testFiles = matcher.GetResultsInFullPath(_recoverCellsAndKzgProofsTests);
        Assert.That(testFiles.Count(), Is.GreaterThan(0));

        foreach (string testFile in testFiles)
        {
            string yaml = File.ReadAllText(testFile);
            RecoverCellsAndKzgProofsTest test = _deserializerUnderscoreNaming.Deserialize<RecoverCellsAndKzgProofsTest>(yaml);
            Assert.That(test, Is.Not.EqualTo(null));

            ulong[] cellIndices = test.Input.CellIndices.ToArray();
            byte[][] cells = GetByteArrays(test.Input.Cells);

            try
            {
                (byte[][] recoveredCells, byte[][] recoveredProofs) = _context.RecoverCellsAndKZGProofs(cellIndices, cells);
                Assert.That(test.Output, Is.Not.EqualTo(null));
                byte[][] expectedCells = GetByteArrays(test.Output.ElementAt(0));
                Assert.That(recoveredCells, Is.EqualTo(expectedCells));
                byte[][] expectedProofs = GetByteArrays(test.Output.ElementAt(1));
                Assert.That(recoveredProofs, Is.EqualTo(expectedProofs));
            }
            catch
            {
                Assert.That(test.Output, Is.EqualTo(null));
            }
        }
    }

    #endregion

    #region EIP-4844 Tests

    #region ComputeKzgProof

    private class ComputeKzgProofInput
    {
        public string Blob { get; set; } = null!;
        public string Z { get; set; } = null!;
    }

    private class ComputeKzgProofTest
    {
        public ComputeKzgProofInput Input { get; set; } = null!;
        public List<string>? Output { get; set; } = null!;
    }

    [TestCase]
    public void TestComputeKzgProof()
    {
        Matcher matcher = new();
        matcher.AddIncludePatterns(new[] { "*/*/data.yaml" });

        IEnumerable<string> testFiles = matcher.GetResultsInFullPath(_computeKzgProofTests);
        Assert.That(testFiles.Count(), Is.GreaterThan(0));

        foreach (string testFile in testFiles)
        {
            string yaml = File.ReadAllText(testFile);
            ComputeKzgProofTest test = _deserializer.Deserialize<ComputeKzgProofTest>(yaml);
            Assert.That(test, Is.Not.EqualTo(null));

            byte[] blob = GetBytes(test.Input.Blob);
            byte[] z = GetBytes(test.Input.Z);

            try
            {
                (byte[] proof, byte[] y) = _context.ComputeKzgProof(blob, z);
                Assert.That(test.Output, Is.Not.EqualTo(null));
                byte[] expectedProof = GetBytes(test.Output[0]);
                byte[] expectedY = GetBytes(test.Output[1]);
                Assert.That(proof, Is.EqualTo(expectedProof));
                Assert.That(y, Is.EqualTo(expectedY));
            }
            catch
            {
                Assert.That(test.Output, Is.EqualTo(null));
            }
        }
    }

    #endregion

    #region ComputeBlobKzgProof

    private class ComputeBlobKzgProofInput
    {
        public string Blob { get; set; } = null!;
        public string Commitment { get; set; } = null!;
    }

    private class ComputeBlobKzgProofTest
    {
        public ComputeBlobKzgProofInput Input { get; set; } = null!;
        public string? Output { get; set; } = null!;
    }

    [TestCase]
    public void TestComputeBlobKzgProof()
    {
        Matcher matcher = new();
        matcher.AddIncludePatterns(new[] { "*/*/data.yaml" });

        IEnumerable<string> testFiles = matcher.GetResultsInFullPath(_computeBlobKzgProofTests);
        Assert.That(testFiles.Count(), Is.GreaterThan(0));

        foreach (string testFile in testFiles)
        {
            string yaml = File.ReadAllText(testFile);
            ComputeBlobKzgProofTest test = _deserializer.Deserialize<ComputeBlobKzgProofTest>(yaml);
            Assert.That(test, Is.Not.EqualTo(null));

            byte[] blob = GetBytes(test.Input.Blob);
            byte[] commitment = GetBytes(test.Input.Commitment);

            try
            {
                byte[] proof = _context.ComputeBlobKzgProof(blob, commitment);
                Assert.That(test.Output, Is.Not.EqualTo(null));
                byte[] expectedProof = GetBytes(test.Output);
                Assert.That(proof, Is.EqualTo(expectedProof));
            }
            catch
            {
                Assert.That(test.Output, Is.EqualTo(null));
            }
        }
    }

    #endregion

    #region VerifyKzgProof

    private class VerifyKzgProofInput
    {
        public string Commitment { get; set; } = null!;
        public string Z { get; set; } = null!;
        public string Y { get; set; } = null!;
        public string Proof { get; set; } = null!;
    }

    private class VerifyKzgProofTest
    {
        public VerifyKzgProofInput Input { get; set; } = null!;
        public bool? Output { get; set; } = null!;
    }

    [TestCase]
    public void TestVerifyKzgProof()
    {
        Matcher matcher = new();
        matcher.AddIncludePatterns(new[] { "*/*/data.yaml" });

        IEnumerable<string> testFiles = matcher.GetResultsInFullPath(_verifyKzgProofTests);
        Assert.That(testFiles.Count(), Is.GreaterThan(0));

        foreach (string testFile in testFiles)
        {
            string yaml = File.ReadAllText(testFile);
            VerifyKzgProofTest test = _deserializer.Deserialize<VerifyKzgProofTest>(yaml);
            Assert.That(test, Is.Not.EqualTo(null));

            byte[] commitment = GetBytes(test.Input.Commitment);
            byte[] z = GetBytes(test.Input.Z);
            byte[] y = GetBytes(test.Input.Y);
            byte[] proof = GetBytes(test.Input.Proof);

            try
            {
                bool isValid = _context.VerifyKzgProof(commitment, z, y, proof);
                Assert.That(isValid, Is.EqualTo(test.Output));
            }
            catch
            {
                Assert.That(test.Output, Is.EqualTo(null));
            }
        }
    }

    #endregion

    #region VerifyBlobKzgProof

    private class VerifyBlobKzgProofInput
    {
        public string Blob { get; set; } = null!;
        public string Commitment { get; set; } = null!;
        public string Proof { get; set; } = null!;
    }

    private class VerifyBlobKzgProofTest
    {
        public VerifyBlobKzgProofInput Input { get; set; } = null!;
        public bool? Output { get; set; } = null!;
    }

    [TestCase]
    public void TestVerifyBlobKzgProof()
    {
        Matcher matcher = new();
        matcher.AddIncludePatterns(new[] { "*/*/data.yaml" });

        IEnumerable<string> testFiles = matcher.GetResultsInFullPath(_verifyBlobKzgProofTests);
        Assert.That(testFiles.Count(), Is.GreaterThan(0));

        foreach (string testFile in testFiles)
        {
            string yaml = File.ReadAllText(testFile);
            VerifyBlobKzgProofTest test = _deserializer.Deserialize<VerifyBlobKzgProofTest>(yaml);
            Assert.That(test, Is.Not.EqualTo(null));

            byte[] blob = GetBytes(test.Input.Blob);
            byte[] commitment = GetBytes(test.Input.Commitment);
            byte[] proof = GetBytes(test.Input.Proof);

            try
            {
                bool isValid = _context.VerifyBlobKzgProof(blob, commitment, proof);
                Assert.That(isValid, Is.EqualTo(test.Output));
            }
            catch
            {
                Assert.That(test.Output, Is.EqualTo(null));
            }
        }
    }

    #endregion

    #region VerifyBlobKzgProofBatch

    private class VerifyBlobKzgProofBatchInput
    {
        public List<string> Blobs { get; set; } = null!;
        public List<string> Commitments { get; set; } = null!;
        public List<string> Proofs { get; set; } = null!;
    }

    private class VerifyBlobKzgProofBatchTest
    {
        public VerifyBlobKzgProofBatchInput Input { get; set; } = null!;
        public bool? Output { get; set; } = null!;
    }

    [TestCase]
    public void TestVerifyBlobKzgProofBatch()
    {
        Matcher matcher = new();
        matcher.AddIncludePatterns(new[] { "*/*/data.yaml" });

        IEnumerable<string> testFiles = matcher.GetResultsInFullPath(_verifyBlobKzgProofBatchTests);
        Assert.That(testFiles.Count(), Is.GreaterThan(0));

        foreach (string testFile in testFiles)
        {
            string yaml = File.ReadAllText(testFile);
            VerifyBlobKzgProofBatchTest test = _deserializer.Deserialize<VerifyBlobKzgProofBatchTest>(yaml);
            Assert.That(test, Is.Not.EqualTo(null));

            byte[][] blobs = GetByteArrays(test.Input.Blobs);
            byte[][] commitments = GetByteArrays(test.Input.Commitments);
            byte[][] proofs = GetByteArrays(test.Input.Proofs);

            try
            {
                bool isValid = _context.VerifyBlobKzgProofBatch(blobs, commitments, proofs);
                Assert.That(isValid, Is.EqualTo(test.Output));
            }
            catch
            {
                Assert.That(test.Output, Is.EqualTo(null));
            }
        }
    }

    #endregion

    #endregion
}