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

        _context = PeerDASKZG.PeerDASContextNew();
        _deserializer = new DeserializerBuilder().WithNamingConvention(CamelCaseNamingConvention.Instance).Build();
        // TODO(Note): On some systems, this is needed as the normal deserializer has trouble deserializing
        // `cell_id` to `CellId` ie the underscore is not being parsed correctly.
        _deserializerUnderscoreNaming = new DeserializerBuilder().WithNamingConvention(UnderscoredNamingConvention.Instance).Build();
    }

    [OneTimeTearDown]
    public void Teardown()
    {
        PeerDASKZG.PeerDASContextFree(_context);
    }

    [TestCase]
    public void TestContextLoaded()
    {
        Assert.That(_context, Is.Not.EqualTo(IntPtr.Zero));
    }

    private IntPtr _context;
    private const string TestDir = "../../../../../../consensus_test_vectors";
    private readonly string _blobToKzgCommitmentTests = Path.Join(TestDir, "blob_to_kzg_commitment");
    private readonly string _computeCellsTests = Path.Join(TestDir, "compute_cells");
    private readonly string _computeCellsAndKzgProofsTests = Path.Join(TestDir, "compute_cells_and_kzg_proofs");
    private readonly string _verifyCellKzgProofTests = Path.Join(TestDir, "verify_cell_kzg_proof");
    private readonly string _verifyCellKzgProofBatchTests = Path.Join(TestDir, "verify_cell_kzg_proof_batch");
    private readonly string _recoverAllCellsTests = Path.Join(TestDir, "recover_all_cells");

    private IDeserializer _deserializer;
    private IDeserializer _deserializerUnderscoreNaming;

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

                commitment = PeerDASKZG.BlobToKzgCommitment(_context, blob);
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

    #region ComputeCells

    private class ComputeCellsInput
    {
        public string Blob { get; set; } = null!;
    }

    private class ComputeCellsTest
    {
        public ComputeCellsInput Input { get; set; } = null!;
        public List<string>? Output { get; set; } = null!;
    }

    [TestCase]
    public void TestComputeCells()
    {
        Matcher matcher = new();
        matcher.AddIncludePatterns(new[] { "*/*/data.yaml" });

        IEnumerable<string> testFiles = matcher.GetResultsInFullPath(_computeCellsTests);
        Assert.That(testFiles.Count(), Is.GreaterThan(0));

        foreach (string testFile in testFiles)
        {
            string yaml = File.ReadAllText(testFile);
            ComputeCellsTest test = _deserializer.Deserialize<ComputeCellsTest>(yaml);
            Assert.That(test, Is.Not.EqualTo(null));

            byte[] cells;
            byte[] blob = GetBytes(test.Input.Blob);

            try
            {
                cells = PeerDASKZG.ComputeCells(_context, blob);
                Assert.That(test.Output, Is.Not.EqualTo(null));
                byte[] expectedCells = GetFlatBytes(test.Output);
                Assert.That(cells, Is.EqualTo(expectedCells));
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
                (byte[] cells, byte[] proofs) = PeerDASKZG.ComputeCellsAndKZGProofs(_context, blob);
                Assert.That(test.Output, Is.Not.EqualTo(null));
                byte[] expectedCells = GetFlatBytes(test.Output.ElementAt(0));
                Assert.That(cells, Is.EqualTo(expectedCells));
                byte[] expectedProofs = GetFlatBytes(test.Output.ElementAt(1));
                Assert.That(proofs, Is.EqualTo(expectedProofs));
            }
            catch
            {
                Assert.That(test.Output, Is.EqualTo(null));
            }
        }
    }

    #endregion

    #region VerifyCellKzgProof

    private class VerifyCellKzgProofInput
    {
        public string Commitment { get; set; } = null!;
        public ulong CellId { get; set; } = 0!;
        public string Cell { get; set; } = null!;
        public string Proof { get; set; } = null!;
    }

    private class VerifyCellKzgProofTest
    {
        public VerifyCellKzgProofInput Input { get; set; } = null!;
        public bool? Output { get; set; } = null!;
    }



    [TestCase]
    public void TestVerifyCellKzgProof()
    {
        Matcher matcher = new();
        matcher.AddIncludePatterns(new[] { "*/*/data.yaml" });

        IEnumerable<string> testFiles = matcher.GetResultsInFullPath(_verifyCellKzgProofTests);
        Assert.That(testFiles.Count(), Is.GreaterThan(0));


        foreach (string testFile in testFiles)
        {
            string yaml = File.ReadAllText(testFile);
            VerifyCellKzgProofTest test = _deserializerUnderscoreNaming.Deserialize<VerifyCellKzgProofTest>(yaml);
            Assert.That(test, Is.Not.EqualTo(null));

            byte[] commitment = GetBytes(test.Input.Commitment);
            ulong cellId = test.Input.CellId;
            byte[] cell = GetBytes(test.Input.Cell);
            byte[] proof = GetBytes(test.Input.Proof);

            try
            {
                bool isCorrect = PeerDASKZG.VerifyCellKZGProof(_context, cell, commitment, cellId, proof);
                Assert.That(isCorrect, Is.EqualTo(test.Output));
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
        public List<string> RowCommitments { get; set; } = null!;
        public List<ulong> RowIndices { get; set; } = null!;
        public List<ulong> ColumnIndices { get; set; } = null!;
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

            byte[] rowCommitments = GetFlatBytes(test.Input.RowCommitments);
            ulong[] rowIndices = test.Input.RowIndices.ToArray();
            ulong[] columnIndices = test.Input.ColumnIndices.ToArray();
            byte[] cells = GetFlatBytes(test.Input.Cells);
            byte[] proofs = GetFlatBytes(test.Input.Proofs);

            try
            {
                bool isCorrect = PeerDASKZG.VerifyCellKZGProofBatch(_context, rowCommitments, rowIndices, columnIndices, cells, proofs);
                Assert.That(isCorrect, Is.EqualTo(test.Output));
            }
            catch
            {
                Assert.That(test.Output, Is.EqualTo(null));
            }
        }
    }

    #endregion

    #region RecoverAllCells

    private class RecoverAllCellsInput
    {
        public List<ulong> CellIds { get; set; } = null!;
        public List<string> Cells { get; set; } = null!;
    }

    private class RecoverAllCellsTest
    {
        public RecoverAllCellsInput Input { get; set; } = null!;
        public List<string>? Output { get; set; } = null!;
    }

    [TestCase]
    public void TestRecoverAllCells()
    {
        Matcher matcher = new();
        matcher.AddIncludePatterns(new[] { "*/*/data.yaml" });

        IEnumerable<string> testFiles = matcher.GetResultsInFullPath(_recoverAllCellsTests);
        Assert.That(testFiles.Count(), Is.GreaterThan(0));

        foreach (string testFile in testFiles)
        {
            string yaml = File.ReadAllText(testFile);
            RecoverAllCellsTest test = _deserializerUnderscoreNaming.Deserialize<RecoverAllCellsTest>(yaml);
            Assert.That(test, Is.Not.EqualTo(null));

            ulong[] cellIds = test.Input.CellIds.ToArray();
            byte[] cells = GetFlatBytes(test.Input.Cells);

            try
            {
                byte[] recoveredCells = PeerDASKZG.RecoverAllCells(_context, cellIds, cells);
                Assert.That(test.Output, Is.Not.EqualTo(null));
                byte[] expectedRecoveredCells = GetFlatBytes(test.Output);
                Assert.That(recoveredCells, Is.EqualTo(expectedRecoveredCells));
            }
            catch
            {
                Assert.That(test.Output, Is.EqualTo(null));
            }
        }
    }

    #endregion
}