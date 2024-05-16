import {ProverContextJs, VerifierContextJs} from "../index.js";

const proverContext = new ProverContextJs();
const verifierContext = new VerifierContextJs();

describe("sum from native", (t) => {
  t.is(sum(1, 2), 3);
});
