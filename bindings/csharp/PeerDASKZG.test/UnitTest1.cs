using PeerDASKZG;

namespace PeerDASKZG.test;

public class Tests
{
    [SetUp]
    public void Setup()
    {
    }

    [Test]
    public void TestSmoke()
    {
        PeerDASKZG.ProverContextNew();
        Assert.AreEqual(true, true);
    }
}