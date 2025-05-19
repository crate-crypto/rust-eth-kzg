# Reed-Solomon erasure recovery via FFTs and vanishing polynomials

## Overview

A Reed-Solomon code is a method for encoding a message (i.e., a polynomial) as a redundant sequence of values (a codeword) such that missing values (erasures) can be recovered—so long as they don't exceed a threshold.

In our [Rust implementation](https://github.com/crate-crypto/rust-eth-kzg/tree/master/crates/cryptography/erasure_codes):
* Encoding uses an FFT over a multiplicative subgroup of a finite field.
* Decoding leverages a *vanishing polynomial* $Z(X)$ to nullify known erasure positions.
* Recovery solves for the original polynomial using division and interpolation.

## Polynomial encoding via FFT

Given a message as a polynomial $f(X)$ of degree less than $n$, we want to expand it into a codeword of length $N = n \cdot r$ for redundancy factor $r$.

We choose an evaluation domain $\mathcal{D} = {\omega^0, \omega^1, \dots, \omega^{N-1}}$, where $\omega$ is a primitive $N$-th root of unity. The codeword is:

$$
[f(\omega^0), f(\omega^1), \dots, f(\omega^{N-1})]
$$

This is computed efficiently using an FFT.

## Erasure recovery intuition

Suppose some positions in the codeword are missing. We denote the received evaluations as $E(X)$, with:
* $E(x_i) = f(x_i)$ if known,
* $E(x_i) = 0$ if missing.

To recover $f(X)$, we construct a vanishing polynomial $Z(X)$ such that:

$$
Z(x_i) = 0 \quad \text{if and only if } x_i \text{ was an erasure}
$$

We then compute the product polynomial $D(X) \cdot Z(X)$ by multiplying $E(X)$ with $Z(X)$ in the evaluation domain. This eliminates the unknowns (because $0 \cdot Z = 0$), and lets us reconstruct $D(X) \cdot Z(X)$ via inverse FFT.

Once we have the product in coefficient form, we evaluate it again over a *coset* domain to safely divide out \$Z(X)\$:

$$
D(X) = \frac{(E \cdot Z)(X)}{Z(X)}
$$

Here, all operations are done in domains where $Z(X)$ has no roots, to avoid division by zero.

The key idea is that $f(X)$ and $D(X)$ agree at all non-erased positions: since $E(x_i) = f(x_i)$ for known $x_i$, and $Z(x_i) \neq 0$ at those points, the product $E(x_i) \cdot Z(x_i)$ equals $f(x_i) \cdot Z(x_i) = D(x_i) \cdot Z(x_i)$. This means $D(X)$ interpolates the same values as $f(X)$ at known points, and hence recovers $f(X)$ exactly.

Finally, we inverse FFT the values of $D(X)$ to recover the coefficient form of $f(X)$.


### Example: Recovering from two erasures — Step by step

Let’s walk through a small Reed-Solomon recovery example.

#### Setup

Suppose we encode a degree-2 polynomial $f(X)$ (i.e., 3 coefficients) into a codeword of length 6, using an expansion factor $r = 2$. So we evaluate $f$ over a domain:

$$
\mathcal{D} = \{x_0, x_1, x_2, x_3, x_4, x_5\}
$$

The encoded codeword is:

$$
f(\mathcal{D}) = [f(x_0), f(x_1), f(x_2), f(x_3), f(x_4), f(x_5)] = [a, b, c, d, e, f]
$$

Now suppose we lose values at $x_1$ and $x_4$. So the received codeword is:

$$
E(X) = [a, 0, c, d, 0, f]
$$

#### Step 1: Construct the vanishing polynomial $Z(X)$

We define $Z(X)$ to vanish exactly at the erasure points $x_1$ and $x_4$:

$$
Z(X) = (X - x_1)(X - x_4)
$$

So:

$$
\begin{aligned}
Z(x_1) &= 0 \\
Z(x_4) &= 0 \\
Z(x_i) &\neq 0 \quad \text{for } i \in \{0,2,3,5\}
\end{aligned}
$$

#### Step 2: Multiply evaluations pointwise

We define:

$$
(E \cdot Z)(x_i) = E(x_i) \cdot Z(x_i)
$$

Let’s annotate each value in the pointwise product:

$$
\begin{aligned}
(E \cdot Z)(x_0) &= a \cdot Z(x_0) \\
(E \cdot Z)(x_1) &= 0 \cdot Z(x_1) = 0 \\
(E \cdot Z)(x_2) &= c \cdot Z(x_2) \\
(E \cdot Z)(x_3) &= d \cdot Z(x_3) \\
(E \cdot Z)(x_4) &= 0 \cdot Z(x_4) = 0 \\
(E \cdot Z)(x_5) &= f \cdot Z(x_5)
\end{aligned}
$$

So the result is:

$$
(E \cdot Z)(X) = [a \cdot Z(x_0), 0, c \cdot Z(x_2), d \cdot Z(x_3), 0, f \cdot Z(x_5)]
$$

#### Step 3: Interpolate $(E \cdot Z)(X)$ via IFFT

We apply inverse FFT to these values over $\mathcal{D}$ to obtain the coefficient representation of the product polynomial:

$$
D(X) \cdot Z(X)
$$

This is the polynomial whose values on $\mathcal{D}$ are the known product evaluations. It "hides" the original $f(X)$ within itself.

#### Step 4: Evaluate on a coset to divide

Now, we evaluate both $D(X) \cdot Z(X)$ and $Z(X)$ on a coset domain $\mathcal{D}' = g \cdot \mathcal{D}$ to avoid zeros in $Z(X)$:

$$
\begin{aligned}
\text{Let } \mathcal{D}' &= \{g x_0, g x_1, \dots, g x_5\} \\
\text{Compute } &\quad (D \cdot Z)(\mathcal{D}') \\
\text{Compute } &\quad Z(\mathcal{D}')
\end{aligned}
$$


Since none of $g x_i$ equals $x_1$ or $x_4$, we are guaranteed that $Z(g x_i) \neq 0$. So we can safely divide pointwise:

$$
D(\mathcal{D}') = \frac{(D \cdot Z)(\mathcal{D}')}{Z(\mathcal{D}')}
$$

#### Step 5: Recover $f(X)$ from $D(\mathcal{D}')$
We now perform inverse FFT on $D(\mathcal{D}')$ over the coset domain to get $D(X)$ in coefficient form:

$$
D(X) = f(X)
$$

Because $D(X) = f(X)$, we recover the original message polynomial.


## Structured erasures: Block synchronization

In many practical settings, erasures are *structured*, such as missing the same position in every block. Suppose we divide the codeword into blocks of size $B$:

$$
[\text{Block}_0, \text{Block}_1, ..., \text{Block}_{m-1}]
$$

If the same index $i$ is missing in every block, we can use this structure to construct $Z(X)$ more efficiently.

Let $R_i$ be the $i$-th root in a smaller domain of size $B$ (the block size). We define the small vanishing polynomial as:

$$
z(X) = \prod_{j \in \text{ missing indices}} (X - R_j)
$$

We then "expand" $z(X)$ to the full domain by inserting zeros in stride: every $k$-th coefficient in $Z(X)$ gets a value from $z(X)$.

This makes $Z(X)$ vanish at the same positions in every block. For example, if blocks are size 4 and we miss index 1 in every block, $Z(X)$ vanishes at:

$$
{1, 5, 9, 13, \dots}
$$

This saves a significant amount of computation when decoding large messages with repeating erasure patterns.



### Example: Block-synchronized erasure recovery — Step by step

Let’s consider a structured erasure scenario where the same position is erased in every block.

## Setup

We take a polynomial $f(X)$ of degree less than 4, and encode it using an expansion factor $r = 2$, so we get a codeword of length:

$$
N = n \cdot r = 4 \cdot 2 = 8
$$

Suppose we divide this codeword into 2 blocks of size $B = 4$:

$$
\begin{aligned}
\text{Block}_0 &= [f(x_0), f(x_1), f(x_2), f(x_3)] \\
\text{Block}_1 &= [f(x_4), f(x_5), f(x_6), f(x_7)]
\end{aligned}
$$

Now imagine the first entry of **every block** is missing. That is:

$$
\text{Erased positions: } x_0 \text{ and } x_4
$$

Our received codeword becomes:

$$
E(X) = [0, f(x_1), f(x_2), f(x_3), 0, f(x_5), f(x_6), f(x_7)]
$$

This is a **block-synchronized erasure** at index $0$ within each block.

#### Step 1: Construct $z(X)$ over the block domain

Let the block domain (of size 4) be:

$$
\mathcal{B} = \{R_0, R_1, R_2, R_3\}
$$

We define a small vanishing polynomial $z(X)$ over the block:

$$
z(X) = X - R_0
$$

because only index 0 is missing within each block. This polynomial satisfies:

$$
z(R_0) = 0, \quad z(R_1), z(R_2), z(R_3) \neq 0
$$

#### Step 2: Expand $z(X)$ to $Z(X)$ over the full domain

We now **lift** $z(X)$ into a polynomial $Z(X)$ that vanishes on index 0 of every block in the full domain. This is done by spacing the coefficients of $z(X)$ apart by block strides (i.e., interleaving zeros between them). If we define:

$$
z(X) = c_0 + c_1 X
$$

Then $Z(X)$ is defined over size 8 by expanding $z(X)$ in strides of 2 (number of blocks):

$$
Z(X) = c_0 + 0 \cdot X + c_1 X^2 + 0 \cdot X^3 + \cdots
$$

This causes $Z(X)$ to vanish at all the original evaluation points $x_i$ such that $i \mod 4 = 0$ (i.e., index 0 in every block). That is:

$$
Z(x_0) = 0, \quad Z(x_4) = 0, \quad Z(x_i) \neq 0 \text{ for all other } i
$$

#### Step 3: Multiply with received codeword

We now compute:

$$
(E \cdot Z)(x_i) = E(x_i) \cdot Z(x_i)
$$

At the erased positions (like $x_0$ and $x_4$), $E(x_i) = 0$, and $Z(x_i) = 0$, so:

$$
(E \cdot Z)(x_0) = (E \cdot Z)(x_4) = 0
$$

At known positions (e.g., $x_1$, $x_2$, ...), $E(x_i) = f(x_i)$ and $Z(x_i) \neq 0$, so the product carries meaningful information.

#### Step 4: Inverse FFT to recover $D(X) \cdot Z(X)$

We now apply inverse FFT to the vector $(E \cdot Z)(x_i)$ to interpolate the product polynomial:

$$
D(X) \cdot Z(X)
$$

#### Step 5: Evaluate on a coset and divide

We choose a coset domain $\mathcal{D}' = g \cdot \mathcal{D}$ and evaluate both $D(X) \cdot Z(X)$ and $Z(X)$ over that coset:

$$
\begin{aligned}
(D \cdot Z)(\mathcal{D}') &= \text{FFT over coset} \\
Z(\mathcal{D}') &= \text{FFT over coset}
\end{aligned}
$$

Because the coset does not contain roots of $Z(X)$, division is safe:

$$
D(\mathcal{D}') = \frac{(D \cdot Z)(\mathcal{D}')}{Z(\mathcal{D}')}
$$

#### Step 6: Final inverse FFT to get $f(X)$

Finally, we inverse FFT $D(\mathcal{D}')$ to get the coefficients of $D(X)$.

Since $D(X)$ agrees with $f(X)$ on all known evaluations, and we constructed $Z(X)$ to remove the effect of erasures, we recover:

$$
D(X) = f(X)
$$


#### Conclusion

We used a **small vanishing polynomial** $z(X)$ and expanded it in a block-aware way to build $Z(X)$ over the full domain, allowing efficient recovery in the presence of repeating erasures.

This strategy reduces the cost of vanishing polynomial construction and FFT operations, making decoding highly scalable.






## Capacity limits

To ensure correct recovery, we need to respect the limits on how many erasures our Reed-Solomon code can tolerate. These limits depend on whether erasures occur randomly or follow a block-synchronized pattern.

### Random erasures

For arbitrary erasure locations, the code can recover as long as we still have at least $n$ known evaluations—just enough to uniquely interpolate a degree- $(n - 1)$ polynomial. Since the full codeword has length $N = n \cdot r$, we can tolerate up to:

$$
N - n = n(r - 1)
$$

random erasures.

This is the classical Reed-Solomon bound: a degree-$(n - 1)$ polynomial is uniquely determined by $n$ distinct evaluations, so losing more than $N - n$ values makes recovery ambiguous or impossible.

### Block-synchronized erasures

Now suppose the codeword is structured into $m$ blocks, each of size $B$, such that:

$$
m = \frac{N}{B}
$$

Assume erasures are synchronized across blocks—for instance, the same $k$ indices are missing from every block. In total, this results in $k \cdot m$ erased values.

To remain recoverable, we must still satisfy:

$$
k \cdot m < N - n
$$

Solving for $k$ gives:

$$
k < \frac{N - n}{m}
$$

Now recall that $N = n \cdot r$ and $m = N / B$, so:

$$
\frac{N - n}{m} = \frac{n(r - 1)}{N / B} = \frac{n(r - 1) \cdot B}{n r} = \frac{(r - 1) \cdot B}{r}
$$


Thus we arrive at:

$$
k < \frac{(r - 1) \cdot B}{r}
$$

And since this bound must be a strict inequality for recovery to succeed, the maximum integer $k$ we can tolerate is:

$$
k < \frac{B}{r}
$$

This means that, for block-synchronized erasures, we can tolerate up to $B / r$ missing indices per block.




## End-to-end recovery algorithm

Here’s a high-level breakdown of the recovery process:

1. Construct $Z(X)$:
   * If erasures are block-synchronized, use block root logic to build a sparse $Z(X)$.
   * If erasures are random, compute the product of $(X - x_i)$ over all missing $x_i$.

2. Compute $E(X) \cdot Z(X)$ via FFT.

3. Interpolate: Inverse FFT to get $(D \cdot Z)(X)$ in coefficient form.

4. Coset division:

   * FFT both numerator and denominator over a coset.
   * Divide pointwise: $D(X) = (E \cdot Z)(X) / Z(X)$.

5. Recover $f(X)$:
   * Inverse FFT to get coefficients of $f(X)$.
   * Verify the degree is $\leq n$.


## Why do we need cosets?

To safely divide $(E \cdot Z)(X)$ by $Z(X)$, we need to avoid the roots of $Z(X)$.

This is where *coset FFTs* come in. We evaluate both numerator and denominator over a shifted domain:

$$
\mathcal{D}' = {g \cdot \omega^i}_{i = 0}^{N - 1}
$$

where $g$ is a coset generator not in the original domain. This ensures $Z(X) \neq 0$ on all points in $\mathcal{D}'$.



## Summary

This Reed-Solomon implementation combines efficient FFT encoding with vanishing polynomial-based decoding. By exploiting erasure structure—particularly block synchronization—it improves recovery time and scalability.

The key ingredients are:
* Polynomial evaluation/interpolation via FFTs
* Sparse vanishing polynomials
* Coset FFTs for safe division
