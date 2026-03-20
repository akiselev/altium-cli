

# Strongly interacting fermions are non-trivial yet non-glassy

Eric R. Anschuetz,<sup>1,2,\*</sup> Chi-Fang Chen,<sup>1,3,†</sup> Bobak T. Kiani,<sup>4,‡</sup> and Robbie King<sup>1,5,§</sup>

<sup>1</sup>*Institute for Quantum Information and Matter, Caltech, Pasadena, CA, USA*

<sup>2</sup>*Walter Burke Institute for Theoretical Physics, Caltech, Pasadena, CA, USA*

<sup>3</sup>*University of California, Berkeley, CA, USA*

<sup>4</sup>*John A. Paulson School of Engineering and Applied Sciences, Harvard, Cambridge, MA, USA*

<sup>5</sup>*Computational and Mathematical Sciences, Caltech, Pasadena, CA, USA*

Random spin systems at low temperatures are glassy and feature computational hardness in finding low-energy states. We study the random all-to-all interacting fermionic Sachdev–Ye–Kitaev (SYK) model and prove that, in contrast, (I) the low-energy states have polynomial circuit depth, yet (II) the annealed and quenched free energies agree to inverse-polynomially low temperatures, ruling out a glassy phase transition in this sense. These results are derived by showing that fermionic and spin systems significantly differ in their *commutation index*, which quantifies the non-commutativity of Hamiltonian terms. Our results suggest that low-temperature strongly interacting fermions, unlike spins, belong in a classically nontrivial yet quantumly easy phase.

## I. INTRODUCTION

Simulating ground and thermal state properties of quantum systems is a key application of future quantum computers [1–7]. Nevertheless, the search for particular, favorable instances that are quantumly easy and classically hard is not clear-cut [8]. A challenge is that current quantum computers are limited in quality and size, requiring the community to rely on theoretical arguments to give computational separations. However, the ground states for standard few-body quantum spin models can be QMA-hard (as classical spin models are NP-hard) in the worst case [9–11]; in the average case, random classical and quantum spin models exhibit glassy physics where computational hardness may arise [12, 13]. To give an efficient quantum algorithm for low-temperature states, one must carefully avoid these instances.

Most chemical and condensed matter systems involve *fermionic* degrees of freedom, not only spins. Of particular importance in quantum chemistry is the *strongly interacting* regime, where Gaussian states do not give good approximations to the ground state and the Hartree–Fock method fails [14]. This has been proposed as a promising regime in which to apply quantum computers to achieve quantum advantage [3]. The Sachdev–Ye–Kitaev (SYK) Hamiltonian provides a natural model for strongly interacting fermions [15–17]. As a counterpart to random spins, it is a random Hamiltonian consisting of all-to-all  $q$ -body Majorana fermions:

$$\mathbf{H}_q^{\text{SYK}} := i^{q/2} \binom{n}{q}^{-1/2} \sum_{j_1 < \dots < j_q} g_{j_1 \dots j_q} \gamma_{j_1} \dots \gamma_{j_q} \quad (1)$$

where  $q$  is assumed to be even,  $\gamma_i \gamma_j + \gamma_j \gamma_i = 2\delta_{ij}$ , and the  $g_{j_1 \dots j_q}$  are i.i.d. standard Gaussian random variables.

While the 4-body fermionic ground state problem can be just as hard as spin models in the worst case

(NP-hard) [18], average-case fermionic systems appear to have qualitatively different physics and perhaps computational complexity than spin systems [13, 19, 20]. Extensive heuristic calculations (such as large- $N$  expansions) together with numerical evidence indicate that the SYK model resembles a thermalizing chaotic system, not a frozen spin-glass as occurs with few-body quantum spin systems [13, 21]. However, rigorous proofs that go beyond the physical arguments have been very limited [19, 22].

In this Letter, we study the strongly interacting SYK model and give quantitative evidence that random, all-to-all connected fermionic systems have a *classically non-trivial yet non-glassy* thermal state at constant temperatures. In contrast, these two properties are false for disordered spin systems [23, 24]. Remarkably, the proofs of both main results rely on the same quantity, the *commutation index* [25]. To bound the commutation index of fermionic operators, we analyze the *Lovasz theta-function* [26] of a certain graph encoding the fermionic commutation relations.

This quantity pinpoints a crucial and often overlooked distinction between fermionic and spin Hamiltonians: low-degree fermionic monomials have a very different commutation structure than low-weight Pauli operators. The commutation index captures this difference, quantifying the fundamental distinction in the physics of local spin systems and local fermionic systems. This disparity, we argue, is the origin of a potential quantum advantage in simulating strongly interacting fermionic systems.

More precisely, we first show that all low-energy states (including constant-temperature thermal states) of the SYK model have high circuit complexity (‘classically non-trivial’):

**Theorem I.1** (Low energy states are classically non-trivial). *Consider the degree- $q$  SYK model  $\mathbf{H}_q^{\text{SYK}}$ . With high probability, the maximum energy is  $\lambda_{\max}(\mathbf{H}_q^{\text{SYK}}) \geq \Omega_q(\sqrt{n})$ , yet any state  $\rho$  such that*

$$\text{Tr}(\rho \mathbf{H}_q^{\text{SYK}}) \geq t\sqrt{n} \quad (2)$$

has circuit complexity

$$\tilde{\Omega}_q(n^{(q/2)+1}t^2). \quad (3)$$

The  $\Omega_q$  notations assume a fixed  $q$  and growing  $n$ .

\* eans@caltech.edu

† achifchen@gmail.com

‡ bkiani@seas.harvard.edu

§ wking@caltech.edu

That is, low-energy states of the SYK model are highly entangled and require many parameters to describe; simple classical ansatzes, such as Gaussian states, must fail. In comparison, local quantum spin systems are known to have efficiently computable product state approximations to the ground state [24] and thus, in this sense, have ‘trivial’ states that achieve a constant-factor approximation of the ground state energy.

Second, we show that the quenched free energy of the SYK model agrees with the annealed free energy even at very low temperatures (‘non-glassy’), formalizing and strengthening previous results of this nature [13, 21, 27, 28].<sup>1</sup> Here, the free energy is normalized such that  $\beta = \mathcal{O}(1)$  corresponds to constant physical temperature.

**Theorem I.2** (Annealed at low temperatures). *Consider the partition function of the degree- $q$  SYK model  $Z_\beta := \text{Tr} \exp(-\beta \sqrt{n} \mathbf{H}_q^{\text{SYK}})$ . Then, we have:*

$$\frac{\mathbb{E} \ln Z_\beta}{n} \leq \frac{\ln \mathbb{E} Z_\beta}{n} \leq \frac{\mathbb{E} \ln Z_\beta}{n} + \mathcal{O}_q(\beta^2 n^{-q/2}). \quad (4)$$

The  $\mathcal{O}_q$  notations assume a fixed  $q$  and growing  $n$ .

The quantitative agreement of the two free energies at (inverse-polynomially) low temperatures strikes a stark contrast with disordered spin systems: the SYK model does not experience a ‘glass’ phase transition in the sense of quenched-vs.-annealed free energy. For classical spin Hamiltonians, it is known that the annealed free energy  $n^{-1} \mathbb{E} \ln Z_\beta$  fails to agree with the quenched free energy  $n^{-1} \ln \mathbb{E} Z_\beta$  at constant temperatures where the Hamiltonian is in its glassy phase and algorithmic hardness arises (see Appendix A); disordered quantum spin systems undergo a similar transition at constant temperature [23]. The lack of a glass transition for the SYK model suggests that there may be no algorithmic obstructions to preparing low-temperature states of the model on a quantum computer, but we do not prove this claim. We leave finding such an efficient quantum algorithm for future work.

Finally, we study the ground state energy of the SYK model as a function of the locality  $q$ , as an extension of Theorem I.1 with potentially large  $q$ .

**Theorem I.3** (Lower bounding the norm with  $q$ -dependence). *For every  $q, n$ , it holds that the maximum energy of the degree- $q$  SYK model is  $\mathbb{E} \lambda_{\max} \geq \Omega(\sqrt{n}/q)$ .*

We show a similar  $\Omega(\sqrt{n}/q)$  scaling for  $q$ -body quantum spin glasses. To our knowledge this is the first lower bound on the maximum eigenvalue which scales as  $\sqrt{n}/q$ ; for both models only the scaling at constant  $q$  was previously known [19, 22, 29, 30]. Our lower bound technique relies on a measure of anticommutation which we call the *commutation degree*, which

counts the maximum number of operators that anticommute with any given operator in the Hamiltonian. Interestingly, the commutation degree does not distinguish between local spin operators and local fermionic operators.

*Background and related work.* The SYK model is a canonical instance of a chaotic Hamiltonian [15–17] with related models studied as far back as [31, 32]. For even  $q = o(\sqrt{n})$ , the SYK model has a Gaussian spectrum [22] and heuristics from physics indicate that the expected maximum energy of the SYK model scales as  $\frac{\sqrt{2n}}{q}$  for even  $q$  [19, 33, 34]. However, the only rigorous result we are aware of with explicit constants is an upper bound of  $\sqrt{\log(2)n}$  [22]. Though Gaussian state approximation algorithms exist for fermionic systems [24, 29], it is known that for the SYK model with  $q \geq 4$ , Gaussian states cannot achieve constant factor approximations to the maximum energy [35]. Separate from the SYK model, so-called no low-energy trivial states (NLTS) theorems rule out constant factor approximations to ground energies with low depth circuits in worst-case settings [36, 37]. For random non-local Hamiltonians, [38] shows a circuit lower bound for sparse, sampled Pauli models using a similar technique as in the proof of Theorem I.1, where the commutation index is much more straightforward to calculate.

The commutation index has connections to other areas of quantum information theory and Hamiltonian complexity. In [39, 40], the commutation index (there termed the *generalized radius*) is used to study generalized Heisenberg uncertainty relations. Related to our work, [19] use the commutation index to analyze the performance of sum-of-squares relaxations of the SYK model and prove the  $q = 4$  instance of Theorem II.1, giving as well an algorithm verifying  $\Omega(\sqrt{n})$  energy for  $q = 4$ . [30] demonstrates that product states maximize the energy variance for random quantum spin Hamiltonians. Finally, the commutation index appears in quantum learning theory, where it provides a sample-complexity lower bound on how many copies of the state are required to learn the expectation values of a set of operators via shadow tomography [25, 41].

## II. COMMUTATION STRUCTURE OF LOCAL OPERATORS

The *commutation index*  $\Delta(\mathcal{S})$  of a given set of operators  $\mathcal{S} = \{\mathbf{A}_1, \dots, \mathbf{A}_m\}$  is defined to be [25]:

$$\Delta(\mathcal{S}) := \sup_{|\psi\rangle} \frac{1}{m} \sum_{i=1}^m \langle \psi | \mathbf{A}_i | \psi \rangle^2. \quad (5)$$

When all  $\|\mathbf{A}_i\| \leq 1$  the commutation index takes values  $0 < \Delta(\mathcal{S}) \leq 1$ . Roughly, a more ‘commuting’ set of observables  $\mathcal{S}$  gives a larger value of  $\Delta(\mathcal{S})$ . For example, if the operators in  $\mathcal{S}$  are all mutually commuting Pauli operators, choosing  $|\psi\rangle$  to be a simultaneous eigenstate gives  $\Delta(\mathcal{S}) = 1$ .

The commutation index has strong implications for the physics of the model  $\mathbf{H} = m^{-1/2} \sum_{i=1}^m g_i \mathbf{A}_i$  with Gaussian coefficients  $g_i$ . Crucially, it controls the sensitivity of many physical properties when varying

<sup>1</sup> In particular, Ref. [13] showed that the SYK model is *consistent* with an annealed approximation, and here we prove that the annealed approximation *holds*.

| Set $\mathcal{S}$     | Commutation index $\Delta(\mathcal{S})$ |
|-----------------------|-----------------------------------------|
| Commuting             | 1                                       |
| $k$ -local Paulis     | $3^{-k}$ (Proposition B.1)              |
| Degree- $q$ Majoranas | $\Theta_q(n^{-q/2})$ (Theorem II.1)     |
| All Paulis            | $2^{-n}$ [41, Lemma 5.8]                |

Table I. The commutation index  $\Delta(\mathcal{S})$  characterizes how non-commuting a set  $\mathcal{S}$  of operators is. The commutation index reveals a key distinction between local spin operators and local fermionic operators: in the fermionic case, the commutation index decays polynomially with system size, while it is constant in the case of spins. The  $\Theta_q$  notation assumes a fixed  $q$  and growing  $n$ .

the couplings of the model. For instance, the norm of the energy gradient of a given state with respect to the disorder is bounded by:

$$\|\nabla_{\mathbf{g}} \langle \phi | \mathbf{H} | \phi \rangle\|_2^2 = \frac{1}{m} \sum_{i=1}^m \langle \phi | \mathbf{A}_i | \phi \rangle^2 \leq \Delta(\mathcal{S}). \quad (6)$$

Our key observation is that the commutation index of the set  $\mathcal{S}_q^n$  of  $\binom{n}{q}$  degree- $q$  Majorana operators is very small:

**Theorem II.1.** *Let  $\mathcal{S}_q^n$  be the set of degree- $q$  Majorana operators on  $n$  fermionic modes. Then for any constant, even  $q$ :*

$$\Delta(\mathcal{S}_q^n) = \Theta_q(n^{-q/2}). \quad (7)$$

The decay with system size  $n$  is unique to the fermionic setting—for local Pauli operators,  $\Delta(\mathcal{S})$  is constant with respect to  $n$  (see Table I). This behaviour was first conjectured in [19] to our knowledge, and we establish the conjecture—including the setting when  $q$  scales with  $n$ —in Appendix B 5.

The proof of Theorem II.1 involves constructing the commutation graph  $G(\mathcal{S})$  whose vertices correspond to operators  $\mathbf{A}_i \in \mathcal{S}$  with edges between operators if and only if they anti-commute. The commutation index can be upper bounded by  $\Delta \leq \vartheta(G(\mathcal{S}))/|G|$ , where  $\vartheta(G(\mathcal{S}))$  is the so-called Lovász theta function of the commutation graph. The Lovász theta function can be efficiently computed via a semi-definite program [26]. For the SYK Hamiltonian,  $G(\mathcal{S}_q^n)$  is the graph of a certain Johnson association scheme [42].

In the course of writing our results we became aware of Ref. [43], which also establishes the necessary results on the Lovász theta function of Johnson association schemes. Our results use different proof techniques and determine the explicit  $q$ -dependence of the constant in Eq. (7), which was not derived in [43].

## III. CIRCUIT LOWER BOUND FOR THE SYK MODEL

An almost direct consequence of a decaying commutation index is a lower bound on the complexity of any ansatz in constructing near-ground states. For any random Hamiltonian  $\mathbf{H} = m^{-1/2} \sum_{i=1}^m g_i \mathbf{A}_i$  with i.i.d. Gaussian coefficients  $g_i$ , the commutation index  $\Delta(\{\mathbf{A}_i\}_{i=1}^m)$  characterizes the maximum variance of

| Ansatz                             | Circuit complexity*                     |
|------------------------------------|-----------------------------------------|
| Quantum circuit with $G$ gates     | $G \geq \tilde{\Omega}_q(n^{q/2+1}t^2)$ |
| MPS with bond dimension $\chi$     | $\chi \geq \Omega_q(n^{q/4+1/2}t)$      |
| Neural network with $W$ parameters | $W \geq \Omega_q(n^{q/2+1}t^2)$         |

\*min. complexity to achieve energy  $t\lambda_{\max}(\mathbf{H}_q^{\text{SYK}})$  w.h.p.

Table II. To achieve energy scaling as  $t\lambda_{\max}(\mathbf{H}_q^{\text{SYK}})$  for the SYK Hamiltonian with high probability, ansatz complexity (e.g., circuit depth) must scale polynomially with  $n$ . See Appendix E for proofs. The  $\Omega_q$  notations assume a fixed  $q$  and growing  $n$ .

the energy  $\langle \psi | \mathbf{H} | \psi \rangle$  for an arbitrary fixed state  $|\psi\rangle$ . Standard concentration bounds then imply that the probability a state  $|\psi\rangle$  has energy  $t$  is bounded as  $\exp(-\Omega(t^2/\Delta))$ . This concentration is so strong that one can bound the maximum energy over extremely large sets of states (or  $\epsilon$ -nets of infinite sets)  $\mathcal{S}$  via a simple union bound argument with high probability over the disorder. In particular, we obtain a lower bound  $|\mathcal{S}| = \exp(\Omega(t^2/\Delta))$  on the cardinality of the class of ansatzes needed to achieve a given energy  $t$ .

Specializing to the SYK model via Theorem I.1, we summarize the implications of this result for various classes of states  $\mathcal{S}$  in Table II. For instance, we show that all states that achieve a constant (i.e.,  $t = \Theta(1)$ ) approximation ratio with the SYK ground state energy have a quantum circuit depth of  $\Omega_q(n^{q/2})$ . In contrast, product states give constant factor approximations to the ground state energy for any local spin Hamiltonian (see Appendix E for a short proof). Our argument also extends to classical ansatzes. For instance, tensor network methods require a bond dimension that grows polynomially with  $n$  to construct near-ground states [44, 45]. Similarly, popular methods based on neural quantum states [46–50] need at least  $\Omega(n^3)$  parameters to construct near-ground states for the standard  $q = 4$  SYK model, implying a bounded depth fully connected network must have layer width that grows as  $\Omega(n^{3/2})$ .

Our circuit lower bound is related to the study of ‘no low-energy trivial states’ (NLTS) Hamiltonians, whose existence was conjectured in [51] and resolved in [36, 37]. However, the settings are not strictly comparable: our instances are random (average-case), whereas NLTS is formalized for worst-case bounded interaction instances of Hamiltonians. The randomness allows us to prove stronger statements in two ways. First, our circuit lower bounds hold for states at *any* constant temperature, rather than for states below some energy threshold. Second, we can achieve arbitrary polynomial circuit depth lower bounds, whereas current constructions of NLTS only give a logarithmic depth lower bound. See Appendix E for more discussion.

## IV. ANNEALED APPROXIMATION FOR THE SYK MODEL

The commutation index also has direct implications for the concentration of various physical properties of interest around their disordered expectation. One manifestation of this is in the relation between the

| Quantity $f$                                     | Rate $K$                                       |
|--------------------------------------------------|------------------------------------------------|
| $\lambda_{\max}(\mathbf{H}_q^{\text{SYK}})$      | $\Omega_q(n^{q/2})$                            |
| $\text{Tr}(\mathbf{X}\rho_\beta)$                | $\Omega_q(\beta^{-2}n^{q/2-1})$                |
| $\text{Tr}(\mathbf{H}_q^{\text{SYK}}\rho_\beta)$ | $\Omega_q(\min(1, \beta^{-2}n^{-2})n^{q/2})^*$ |

\*for  $t$  order of  $\|\mathbf{H}_q^{\text{SYK}}\| = \mathcal{O}(\sqrt{n})$

Table III. Concentration bounds for functions  $f$  of the Hamiltonian around its mean, i.e.,  $\mathbb{P}[|f - \mathbb{E}[f]| \geq t] \leq 4\exp(-Kt^2)$ .  $\lambda_{\max}$  denotes the largest eigenvalue and  $\mathbf{X}$  is an arbitrary bounded operator.  $\rho_\beta$  is the thermal state of  $\sqrt{n}\mathbf{H}$  at an inverse temperature  $\beta$ .

quenched and annealed free energies:

$$\underbrace{\frac{1}{n}\mathbb{E}\ln Z_\beta}_{\text{quenched}} \leq \underbrace{\frac{1}{n}\ln \mathbb{E}Z_\beta}_{\text{annealed}}, \quad (8)$$

where  $Z_\beta$  is the partition function of the model  $\sqrt{n}\mathbf{H}$  at an inverse temperature  $\beta$ . The quenched free energy assumes the disorder induced by the random couplings is fixed when averaging over thermal fluctuations; the annealed free energy treats these fluctuations on an equal footing. While the two quantities agree at high temperature, at low temperature the latter is incapable of accounting for frustration induced by the disorder of the random couplings which can induce a spin glass phase [52, 53]. Their disagreement is thus indicative of the presence of a spin glass phase (see Appendix A). Motivated by this we bound the difference in quenched and annealed free energies as a function of the temperature and the commutation index of the model:

$$\frac{1}{n}\mathbb{E}\ln Z_\beta \leq \frac{1}{n}\ln \mathbb{E}Z_\beta \leq \frac{1}{n}\mathbb{E}\ln Z_\beta + 4\beta^2\Delta. \quad (9)$$

For the SYK model this directly implies Theorem I.2. Informally, this bound is due to controlling the growth of the moment generating function of  $\ln(Z_\beta) - \mathbb{E}[\ln(Z_\beta)]$  using the commutation index  $\Delta$ . We formally prove Eq. (9) in Appendix C. We there also prove concentration bounds for observable expectations as well as two-point correlators, again following from bounding how sensitive these quantities are when varying the disorder. We summarize some of these results when applied to the SYK model in Table III.

## V. LOWER BOUND FOR THE SYK MAXIMUM EIGENVALUE

Finally, we show how commutation properties of local Hamiltonians can give rise to lower bounds on the maximum eigenvalue. In particular, for any random Hamiltonian  $\mathbf{H} = m^{-1/2} \sum_{i=1}^m g_i \mathbf{A}_i$  where all  $\mathbf{A}_i^2 = \mathbf{I}$ , the scaling of the maximum eigenvalue is related to what we call the *commutation degree*  $h_{\text{comm}}(\{\mathbf{A}_i\}_{i=1}^m)$

of the operators comprising the Hamiltonian:

$$h_{\text{comm}}(\{\mathbf{A}_i\}_{i=1}^m) := \frac{1}{2} \sup_i \sum_{j=1}^m \|\mathbf{A}_i, \mathbf{A}_j\|. \quad (10)$$

The name ‘commutation degree’ is derived from the fact that it is the maximal degree of the commutation graph associated with the set  $\mathcal{S} = \{\mathbf{A}_1, \dots, \mathbf{A}_m\}$ ; see Appendix B 2. The commutation degree can be interpreted as controlling the maximal amount of operator spreading of any  $\mathbf{A}_i$  under the dynamics of  $\mathbf{H}$ . Using this intuition we are able to control how sensitive the expected partition function is to the inverse temperature:

$$\frac{\partial}{\partial \beta} \mathbb{E} \text{Tr}[e^{\beta \mathbf{H}}] \geq \beta \left(1 - \frac{c_1 \beta^2 h_{\text{comm}}}{m}\right) \mathbb{E} \text{Tr}[e^{\beta \mathbf{H}}], \quad (11)$$

where  $c_1 > 0$  is an absolute constant. Using the lower bound of  $\text{Tr}[e^{\beta \mathbf{H}}] \leq \exp(\beta O(n) + \beta \lambda_{\max}(\mathbf{H}))$ , we prove in Appendix D that, for all  $\beta$ ,

$$\mathbb{E} \exp(\beta \lambda_{\max}(\mathbf{H})) \geq \exp\left(\frac{\beta^2}{2} \left(1 - \frac{c_1 \beta^2 h_{\text{comm}}}{2m}\right)\right). \quad (12)$$

Using the fact that the maximal eigenvalue concentrates (as in, e.g., Table III) and maximizing the bound over  $\beta$  then implies:

$$\mathbb{E} \lambda_{\max}(\mathbf{H}) \geq \frac{\sqrt{m}}{4\sqrt{c_1 h_{\text{comm}}}} (1 - 16\Delta). \quad (13)$$

By the same concentration results this bound also holds for  $\lambda_{\max}(\mathbf{H})$  with high probability over the disorder, not just in expectation. Intriguingly, the overall scaling depends only on the commutation degree  $h_{\text{comm}}$ . This quantity (when normalized by  $m$ ) agrees to leading order for both the  $q$ -body quantum spin glass model  $\mathbf{H}_q^{\text{SG}}$  and the SYK model  $\mathbf{H}_q^{\text{SYK}}$ . As long as  $\Delta < 1/16$ —which is true for the former when  $q \geq 3$ , and always for the latter for sufficiently large  $n$ —this implies that for *both* models:

$$\lambda_{\max}(\mathbf{H}_q^{\text{SYK}}) \geq \Omega\left(\frac{\sqrt{n}}{q}\right), \quad \lambda_{\max}(\mathbf{H}_q^{\text{SG}}) \geq \Omega\left(\frac{\sqrt{n}}{q}\right). \quad (14)$$

We here allow the locality  $q$  to potentially grow with  $n$  as  $n \rightarrow \infty$ . This strengthens the previously-known  $\Omega(\sqrt{n})$  scaling for both when  $q$  is constant [19, 29, 30]. For  $q = \omega(\sqrt{n})$  it is known that both models exhibit a phase transition in their spectrums to a semicircle law [22, 54] which is consistent with our results.

## ACKNOWLEDGMENTS

E.R.A. is funded in part by the Walter Burke Institute for Theoretical Physics at Caltech. C.-F.C. is supported by a Simons-CIQC postdoctoral fellowship through NSF QLCI Grant No. 2016245. R.K. is funded by NSF grant CCF-2321079. We thank Chokri Manai, who pointed out an error in an earlier draft and shared his insights on its solution.

- 
- [1] R. P. Feynman, Simulating physics with computers, *International Journal of Theoretical Physics* **21**, 467 (1982).
- [2] S. Lloyd, Universal quantum simulators, *Science* **273**, 1073 (1996).
- [3] S. McArdle, S. Endo, A. Aspuru-Guzik, S. C. Benjamin, and X. Yuan, Quantum computational chemistry, *Rev. Mod. Phys.* **92**, 015003 (2020).
- [4] J. Lee, D. W. Berry, C. Gidney, W. J. Huggins, J. R. McClean, N. Wiebe, and R. Babbush, Even more efficient quantum computations of chemistry through tensor hypercontraction, *PRX Quantum* **2**, 030305 (2021).
- [5] V. von Burg, G. H. Low, T. Häner, D. S. Steiger, M. Reiher, M. Roetteler, and M. Troyer, Quantum computing enhanced computational catalysis, *Physical Review Research* **3**, 033055 (2021).
- [6] R. Babbush, N. Wiebe, J. McClean, J. McClain, H. Neven, and G. K.-L. Chan, Low-depth quantum simulation of materials, *Phys. Rev. X* **8**, 011044 (2018).
- [7] C. Chamberland, K. Noh, P. Arrangoiz-Arriola, E. T. Campbell, C. T. Hann, J. Iverson, H. Putterman, T. C. Bohdanowicz, S. T. Flammia, A. Keller, G. Refael, J. Preskill, L. Jiang, A. H. Safavi-Naeini, O. Painter, and F. G. S. L. Brandão, Building a fault-tolerant quantum computer using concatenated cat codes (2020), [arXiv:2012.04108 \[quant-ph\]](#).
- [8] S. Lee, J. Lee, H. Zhai, Y. Tong, A. M. Dalzell, A. Kumar, P. Helms, J. Gray, Z.-H. Cui, W. Liu, M. Kastoryano, R. Babbush, J. Preskill, D. R. Reichman, E. T. Campbell, E. F. Valeev, L. Lin, and G. K.-L. Chan, Is there evidence for exponential quantum advantage in quantum chemistry? (2022).
- [9] A. Y. Kitaev, A. Shen, M. N. Vyalyi, and M. N. Vyalyi, *Classical and quantum computation*, 47 (American Mathematical Soc., 2002).
- [10] D. Aharonov, D. Gottesman, S. Irani, and J. Kempe, The power of quantum systems on a line, *Communications in mathematical physics* **287**, 41 (2009).
- [11] D. Gottesman and S. Irani, The quantum and classical complexity of translationally invariant tiling and Hamiltonian problems, in *2009 50th Annual IEEE Symposium on Foundations of Computer Science* (IEEE, 2009) pp. 95–104.
- [12] D. Gamarnik, The overlap gap property: A topological barrier to optimizing over random structures, *Proceedings of the National Academy of Sciences* **118**, e2108492118 (2021).
- [13] C. L. Baldwin and B. Swingle, Quenched vs annealed: Glassiness from SK to SYK, *Phys. Rev. X* **10**, 031026 (2020).
- [14] A. Szabo and N. S. Ostlund, *Modern quantum chemistry: introduction to advanced electronic structure theory* (Courier Corporation, 2012).
- [15] S. Sachdev and J. Ye, Gapless spin-fluid ground state in a random quantum heisenberg magnet, *Physical review letters* **70**, 3339 (1993).
- [16] A. Kitaev, Hidden correlations in the hawking radiation and thermal noise, Talk at Kavli Institute for Theoretical Physics (2015).
- [17] A. Kitaev, A simple model of quantum holography, Talks at Kavli Institute for Theoretical Physics (2015).
- [18] Y.-K. Liu, M. Christandl, and F. Verstraete, Quantum computational complexity of the n-representability problem: Qma complete, *Physical review letters* **98**, 110503 (2007).
- [19] M. B. Hastings and R. O’Donnell, Optimizing strongly interacting fermionic hamiltonians, in *Proceedings of the 54th Annual ACM SIGACT Symposium on Theory of Computing* (2022) pp. 776–789.
- [20] J. Maldacena and D. Stanford, Comments on the Sachdev-Ye-Kitaev model, *arXiv preprint arXiv:1604.07818* **19** (2016).
- [21] D. Facoetti, G. Biroli, J. Kurchan, and D. R. Reichman, Classical glasses, black holes, and strange quantum liquids, *Physical Review B* **100**, 205108 (2019).
- [22] R. Feng, G. Tian, and D. Wei, Spectrum of SYK model, *Peking Mathematical Journal* **2**, 41 (2019).
- [23] C. Baldwin and B. Swingle, Quenched vs annealed: Glassiness from SK to SYK, *Physical Review X* **10**, 031026 (2020).
- [24] S. Bravyi, D. Gosset, R. König, and K. Temme, Approximation algorithms for quantum many-body problems, *Journal of Mathematical Physics* **60** (2019).
- [25] R. King, D. Gosset, R. Kothari, and R. Babbush, Triply efficient shadow tomography, *arXiv preprint arXiv:2404.19211* (2024).
- [26] D. E. Knuth, The sandwich theorem, *arXiv preprint math/9312214* (1993).
- [27] R. Gurau, Quenched equals annealed at leading order in the colored syk model, *Europhysics letters* **119**, 30003 (2017).
- [28] A. Georges, O. Parcollet, and S. Sachdev, Mean field theory of a quantum heisenberg spin glass, *Physical review letters* **85**, 840 (2000).
- [29] Y. Herasymenko, M. Stroecks, J. Helsen, and B. Terhal, Optimizing sparse fermionic hamiltonians, *Quantum* **7**, 1081 (2023).
- [30] E. R. Anschuetz, D. Gamarnik, and B. T. Kiani, Bounds on the ground state energy of quantum  $p$ -spin hamiltonians, *arXiv preprint arXiv:2404.07231* (2024).
- [31] J. French and S. Wong, Validity of random matrix theories for many-particle systems, *Physics Letters B* **33**, 449 (1970).
- [32] O. Bohigas and J. Flores, Two-body random hamiltonian and level density, *Physics Letters B* **34**, 261 (1971).
- [33] A. M. García-García, Y. Jia, and J. J. Verbaarschot, Exact moments of the Sachdev-Ye-Kitaev model up to order  $1/n^2$ , *Journal of High Energy Physics* **2018**, 1 (2018).
- [34] A. M. García-García and J. J. Verbaarschot, Spectral and thermodynamic properties of the Sachdev-Ye-Kitaev model, *Physical Review D* **94**, 126010 (2016).
- [35] A. Haldar, O. Tavakol, and T. Scaffidi, Variational wave functions for Sachdev-Ye-Kitaev models, *Physical Review Research* **3**, 023020 (2021).

- [36] A. Anshu, N. P. Breuckmann, and C. Nirkhe, Nits hamiltonians from good quantum codes, in *Proceedings of the 55th Annual ACM Symposium on Theory of Computing* (2023) pp. 1090–1096.
- [37] Y. Herasymenko, A. Anshu, B. Terhal, and J. Helsen, Fermionic hamiltonians without trivial low-energy states, arXiv preprint arXiv:2307.13730 (2023).
- [38] C.-F. A. Chen, A. M. Dalzell, M. Berta, F. G. Brandão, and J. A. Tropp, Sparse random hamiltonians are quantumly easy (2023).
- [39] C. de Gois, K. Hansenne, and O. Gühne, Uncertainty relations from graph theory, *Physical Review A* **107**, 062211 (2023).
- [40] Z.-P. Xu, R. Schwonnek, and A. Winter, Bounding the joint numerical range of Pauli strings by graph parameters, arXiv preprint arXiv:2308.00753 (2023).
- [41] S. Chen, J. Cotler, H.-Y. Huang, and J. Li, Exponential separations between learning with and without quantum memory, in *2021 IEEE 62nd Annual Symposium on Foundations of Computer Science (FOCS)* (IEEE, 2022) pp. 574–585.
- [42] P. Delsarte, An algebraic approach to the association schemes of coding theory, *Philips Res. Rep. Suppl.* **10**, vi+ (1973).
- [43] W. Linz,  $L$ -systems and the Lovász number, arXiv preprint arXiv:2402.05818 (2024).
- [44] U. Schollwöck, The density-matrix renormalization group in the age of matrix product states, *Annals of physics* **326**, 96 (2011).
- [45] M. C. Bañuls, Tensor network algorithms: A route map, *Annual Review of Condensed Matter Physics* **14**, 173 (2023).
- [46] G. Carleo and M. Troyer, Solving the quantum many-body problem with artificial neural networks, *Science* **355**, 602 (2017).
- [47] M. Schmitt and M. Heyl, Quantum many-body dynamics in two dimensions with artificial neural networks, *Physical Review Letters* **125**, 100503 (2020).
- [48] O. Sharir, Y. Levine, N. Wies, G. Carleo, and A. Shashua, Deep autoregressive models for the efficient variational simulation of many-body quantum systems, *Physical review letters* **124**, 020503 (2020).
- [49] O. Sharir, A. Shashua, and G. Carleo, Neural tensor contractions and the expressive power of deep neural quantum states, *Physical Review B* **106**, 205136 (2022).
- [50] Y. Nomura and M. Imada, Dirac-type nodal spin liquid revealed by refined quantum many-body solver using neural-network wave function, correlation ratio, and level spectroscopy, *Physical Review X* **11**, 031034 (2021).
- [51] M. H. Freedman and M. B. Hastings, Quantum systems on non- $k$ -hyperfinite complexes: A generalization of classical statistical mechanics on expander graphs, arXiv preprint arXiv:1301.1363 (2013).
- [52] M. Talagrand, Rigorous low-temperature results for the mean field  $p$ -spins interaction model, *Probability theory and related fields* **117**, 303 (2000).
- [53] G. Parisi, Infinite number of order parameters for spin-glasses, *Physical Review Letters* **43**, 1754 (1979).
- [54] L. Erdős and D. Schröder, Phase transition in the density of states of quantum spin glasses, *Mathematical Physics, Analysis and Geometry* **17**, 441 (2014).
- [55] T. R. Kirkpatrick and D. Thirumalai,  $p$ -spin-interaction spin-glass models: Connections with the structural glass problem, *Physical Review B* **36**, 5388 (1987).
- [56] D. Gamarnik, A. Jagannath, and E. C. Kızıldağ, Shattering in the ising pure  $p$ -spin model, arXiv preprint arXiv:2307.07461 (2023).
- [57] B. Derrida, Random-energy model: Limit of a family of disordered models, *Physical Review Letters* **45**, 79 (1980).
- [58] G. Gur-Ari, R. Mahajan, and A. Vaezi, Does the syk model have a spin glass phase?, *Journal of High Energy Physics* **2018**, 1 (2018).
- [59] H. Leschke, C. Manai, R. Ruder, and S. Warzel, Existence of replica-symmetry breaking in quantum glasses, *Physical Review Letters* **127**, 207204 (2021).
- [60] C. Manai and S. Warzel, A parisi formula for quantum spin glasses, arXiv preprint arXiv:2403.06155 (2024).
- [61] D. Gamarnik, A. Jagannath, and A. S. Wein, Circuit lower bounds for the  $p$ -spin optimization problem, arXiv preprint arXiv:2109.01342 (2021).
- [62] B. Huang and M. Sellke, Tight lipschitz hardness for optimizing mean field spin glasses, in *2022 IEEE 63rd Annual Symposium on Foundations of Computer Science (FOCS)* (IEEE, 2022) pp. 312–322.
- [63] B. Huang and M. Sellke, Algorithmic threshold for multi-species spherical spin glasses, arXiv preprint arXiv:2303.12172 (2023).
- [64] A. Y. Vlasov, Clifford algebras, spin groups and qubit trees, arXiv preprint arXiv:1904.09912 (2019).
- [65] Z. Jiang, A. Kalev, W. Mruczkiewicz, and H. Neven, Optimal fermion-to-qubit mapping via ternary trees with applications to reduced quantum states learning, *Quantum* **4**, 276 (2020).
- [66] P. Borwein and T. Erdelyi, *Polynomials and Polynomial Inequalities*, Graduate Texts in Mathematics, Vol. 161 (Springer, New York, 1995).
- [67] C.-F. Chen, J. Garza-Vargas, J. A. Tropp, and R. van Handel, A new approach to strong convergence, arXiv preprint arXiv:2405.16026 (2024).
- [68] C.-F. Chen, A. Bouland, F. G. Brandão, J. Docter, P. Hayden, and M. Xu, Efficient unitary designs and pseudorandom unitaries from permutations, arXiv preprint arXiv:2404.16751 (2024).
- [69] A. Kitaev and S. J. Suh, The soft mode in the Sachdev-Ye-Kitaev model and its gravity dual, *Journal of High Energy Physics* **2018**, 1 (2018).
- [70] J. Maldacena and D. Stanford, Remarks on the Sachdev-Ye-Kitaev model, *Physical Review D* **94**, 106002 (2016).
- [71] R. Feng, G. Tian, and D. Wei, Spectrum of SYK model III: large deviations and concentration of measures, *Random Matrices: Theory and Applications* **9**, 2050001 (2020).
- [72] M. J. Wainwright, Basic tail and concentration bounds, in *High-Dimensional Statistics: A Non-Asymptotic Viewpoint*, Cambridge Series in Statistical and Probabilistic Mathematics (Cambridge University Press, 2019) pp. 21–57.
- [73] A. S. Bandeira, M. T. Boedihardjo, and R. van Handel, Matrix concentration inequalities and free probability (2021).

- [74] P. Rigollet and J.-C. Hütter, High-dimensional statistics, arXiv preprint arXiv:2310.19244 (2023).
- [75] R. Vershynin, *High-Dimensional Probability: An Introduction with Applications in Data Science*, Vol. 47 (Cambridge University Press, 2018).
- [76] M. J. Wainwright, *High-dimensional statistics: A non-asymptotic viewpoint*, Vol. 48 (Cambridge university press, 2019).
- [77] R. M. Wilcox, Exponential operators and parameter differentiation in quantum physics, *Journal of Mathematical Physics* **8**, 962 (1967).
- [78] R. Bhatia, *Matrix Analysis* (Springer, 1997).
- [79] A. M. Dalzell, M. Berta, F. G. Brandão, J. A. Tropp, *et al.*, Sparse random hamiltonians are quantumly easy, arXiv preprint arXiv:2302.03394 (2023).
- [80] A. Haldar, O. Tavakol, and T. Scaffidi, Variational wave functions for Sachdev-Ye-Kitaev models, *Phys. Rev. Res.* **3**, 023020 (2021).
- [81] L. Eldar and A. W. Harrow, Local hamiltonians whose ground states are hard to approximate, in *2017 IEEE 58th Annual Symposium on Foundations of Computer Science (FOCS)* (2017) pp. 427–438.
- [82] A. Anshu and N. P. Breuckmann, A construction of combinatorial NLTS, *Journal of Mathematical Physics* **63**, 122201 (2022).
- [83] E. R. Anschuetz, D. Gamarnik, and B. Kiani, Combinatorial NLTS from the overlap gap property, arXiv preprint arXiv:2304.00643 (2023).
- [84] H.-Y. Huang, R. Kueng, and J. Preskill, Predicting many properties of a quantum system from very few measurements, *Nature Physics* **16**, 1050 (2020).

## Appendix A: Background on related results for classical spin glass models

Spin glass models are a now-canonical object in the study of disordered systems in physics and mathematics. Properties of the energy landscape of this model feature phase transitions which govern the complexity of the near ground states and have various algorithmic and physical implications. We briefly summarize these important implications here, focusing on the (classical) Ising spin glass model.

In the Ising spin model, configurations  $\sigma = (\sigma_i)_{i=1}^n$  are each a point in the space  $\Sigma = \{-1, +1\}^n$  with energy given by the random Hamiltonian

$$H_C(\sigma) = \frac{1}{\sqrt{\binom{n}{p}}} \sum_{1 \leq i_1 < \dots < i_p \leq n} J_{i_1 \dots i_p} \sigma_{i_1} \cdots \sigma_{i_p}, \quad (\text{A1})$$

where  $J_{i_1 \dots i_p} \sim \mathcal{N}(0, 1)$  are coefficients drawn i.i.d. from the standard Normal distribution. This model can equivalently be viewed as a qubit or matrix model on the vector space  $\mathbb{C}^{2^n}$ . Denoting  $Z_i$  as the Pauli Z matrix acting on qubit  $i$ , the Hamiltonian  $H_C$  takes the form

$$H_C = \frac{1}{\sqrt{\binom{n}{p}}} \sum_{1 \leq i_1 < \dots < i_p \leq n} J_{i_1 \dots i_p} Z_{i_1} \cdots Z_{i_p}. \quad (\text{A2})$$

Note that the eigenvectors of the above Hamiltonian map onto spins in Eq. A1 with energy given by the corresponding eigenvalue. For this model, the (quenched) free energy takes the form

$$F(\beta) = n^{-1} \mathbb{E} [\log Z_\beta], \quad Z_\beta = \sum_{\sigma \in \Sigma} \exp(\beta \sqrt{n} H_C(\sigma)), \quad (\text{A3})$$

with associated Gibbs measure

$$\mu_\beta(A) = Z_\beta^{-1} \sum_{\sigma \in A} \exp(-\beta \sqrt{n} H_C(\sigma)). \quad (\text{A4})$$

Given that for any  $\sigma$  the random variable  $H_C(\sigma)$  is distributed as Gaussian with unit variance, one can show that the limiting annealed free energy is

$$\lim_{n \rightarrow \infty} n^{-1} \log \mathbb{E} [Z_\beta] = \lim_{n \rightarrow \infty} n^{-1} \log (2^n \exp(\beta^2 n/2)) = \log(2) + \beta^2/2. \quad (\text{A5})$$

Explicitly calculating the quenched free energy is far more challenging. Nonetheless, Kirkpatrick and Thirumalai [55] predicted that below a critical temperature, the structure of the Gibbs distribution clusters into exponentially many disconnected clusters indicative of a so-called shattering phase of a spin model. Such a shattering phase is a characteristic of ‘glassiness’ and a source of formal proofs for algorithmic hardness of finding near-ground states. We summarize these findings below.

Evidence of shattering was provided in [52] and only recently fully rigorously proven in [56] for the large  $p$  limit. The phase transition coincides with the point where the quenched and annealed free energies fail to agree, formalized in [52] as:

$$\beta_p := \sup \left\{ \beta : \limsup_{n \rightarrow \infty} \frac{\mathbb{E}[\log Z_\beta]}{n} = \log(2) + \frac{\beta^2}{2} \right\}. \quad (\text{A6})$$

Talagrand shows in Theorem 1.1 of [52] that

$$(1 - 2^{-p})\sqrt{2\log(2)} \leq \beta_p \leq \sqrt{2\log(2)}. \quad (\text{A7})$$

$\beta_p \rightarrow \sqrt{2\log(2)}$  in the limit  $p \rightarrow \infty$  and is notable for also being the transition point where the quenched and annealed free energies fail to agree for the Random Energy Model [57]. In the Random Energy Model, the  $2^n$  different configurations each have an energy given by independent draws of a Gaussian random variable with variance  $n$ . In studying potential ‘glassiness’ in SYK energy landscapes, [58] study the distribution of low energy eigenvalues of SYK model. There, numerical evidence is presented that extremal eigenvalues of the SYK model have level repulsion, which is also observed in many random matrix theory eigenspectra but which is not a feature of the Random Energy Model.

Failure of the quenched free energy to agree with the annealed free energy is typically an indication that an exponentially small fraction of low energy states dominate contributions to the Gibbs distribution. Note that the quenched free energy for a particular draw of the coefficients is typically close to its expectation (i.e., self-averaging in the terminology of statistical mechanics), but this is not true of the partition function  $Z_\beta$ . Fluctuations in low energy states can cause  $Z_\beta$  to oscillate significantly. When these outliers dominate the contribution to the Gibbs distribution, the annealed free energy can disagree with the quenched free energy.

The point  $\beta_p$  intuitively indicates the transition into a regime where low-energy states are rare but nonetheless dominate the contribution to the Gibbs distribution. This fact has many implications. [52] shows that whether  $\beta$  is greater or less than  $\beta_p$  determines whether or not ‘overlaps’  $R(\sigma, \sigma')$  of configurations  $\sigma, \sigma'$  drawn from the Gibbs distribution converge to zero, where

$$R(\sigma, \sigma') = n^{-1} \sum_i \sigma_i \sigma'_i. \quad (\text{A8})$$

More explicitly, treating  $R(\sigma, \sigma')$  as a random variable where  $\sigma, \sigma'$  are drawn independently from the Gibbs measure at temperature  $\beta$ , Talagrand shows in Proposition 1.2 of [52] that

$$\beta < \beta_p \implies \lim_{n \rightarrow \infty} \mathbb{E} \langle R(\sigma, \sigma')^2 \rangle_\beta = 0 \quad (\text{A9})$$

$$\beta > \beta_p \implies \exists \beta' < \beta, \quad \limsup_{N \rightarrow \infty} \mathbb{E} \langle R(\sigma, \sigma')^2 \rangle_{\beta'} > 0 \quad (\text{A10})$$

$$\beta > \sqrt{2\log 2} \implies \liminf_{N \rightarrow \infty} \mathbb{E} \langle R(\sigma, \sigma')^2 \rangle_\beta > 0, \quad (\text{A11})$$

where  $\langle \cdot \rangle_\beta$  denotes the thermal or Gibbs average over independent replicas.  $\liminf_{N \rightarrow \infty} \mathbb{E} \langle R(\sigma, \sigma')^2 \rangle_\beta > 0$  is the defining property of the glassy phase of a system, so this formally connects the disagreement of the quenched and annealed free energy with the onset of glassiness. Similar replica symmetry breaking behavior has also been proven to exist for Sherrington–Kirkpatrick models with a quantum transverse field (i.e.,  $\sum_i \sigma_i^x$ ) of sufficiently low strength [59, 60]. Though we do not detail it here, for many glassy systems this transition is also associated with the onset of a clustering phenomenon in the landscape of low energy states known as the *Overlap Gap Property* (OGP) [12]. This property is known to be the cause of the algorithmic hardness of optimizing glassy systems [61–63].

We now turn toward the SYK model. Let us denote the corresponding value of  $\beta_p$  for the SYK model as

$$\beta_p^{\text{SYK}} := \sup \left\{ \beta : \limsup_{n \rightarrow \infty} \frac{\mathbb{E}[\log Z_\beta]}{n} = \lim_{n \rightarrow \infty} \frac{\log \mathbb{E} Z_\beta}{n} \right\}, \quad (\text{A12})$$

where the free energy above is that of the SYK model and we assume the limit above exists for the annealed free energy. We show in Theorem C.1 that for  $q \geq 4$ ,  $\beta_p^{\text{SYK}}$  must grow with  $n$  and the transition governed by  $\beta_p^{\text{SYK}}$  cannot occur at any constant temperature. Insofar as the story from the spin glass setting agrees with that of the SYK model this would imply the lack of a transition into a clustered phase with topological barriers to optimization. Nonetheless, formalizing such notions of ‘glassiness’ and the OGP for SYK models appear to be formidable tasks.

## Appendix B: Commutation index and the Lovász theta function

### 1. Commutation index

In this section we study a property which quantifies the commutation structure of a set of operators. This allows us to discuss the commutation structure of local Majorana operators and how they differ from local Paulis.

**Definition B.1.** For a set  $\mathcal{S}$  of Hermitian operators, define their commutation index by

$$\Delta(\mathcal{S}) = \sup_{|\psi\rangle} \mathbb{E}_{\mathbf{A} \in \mathcal{S}} \langle \psi | \mathbf{A} | \psi \rangle^2. \quad (\text{B1})$$

The commutation index of the set of  $k$ -local Paulis is independent of system size  $n$ .

**Proposition B.1.** Let  $\mathcal{P}_k^n$  be the set of  $k$ -local  $n$ -qubit Paulis. When  $2n + 1 \geq 3^k$ , it holds:

$$\Delta(\mathcal{P}_k^n) = 3^{-k}. \quad (\text{B2})$$

Moreover, the maximum is achieved by any product state.

We also prove a slightly weaker bound that holds for any  $k$ .

**Proposition B.2.** Let  $\mathcal{P}_k^n$  be the set of  $k$ -local  $n$ -qubit Paulis. Then, for any  $k, n$ , it holds:

$$\Delta(\mathcal{P}_k^n) \leq \left(\frac{3}{2}\right)^{-k}. \quad (\text{B3})$$

See [Appendix B 3](#) for proofs of [Proposition B.1](#) and [Proposition B.2](#).

On the other hand, the commutation index of the set of degree- $q$  Majorana operators decays polynomially with system size.

**Definition B.2.** The Majorana operators on  $n$  fermionic modes are defined abstractly as  $n$  operators  $\{\gamma_1, \dots, \gamma_n\}$  which satisfy the relations

$$\gamma_a \gamma_b + \gamma_b \gamma_a = 2\delta_{ab} \mathbb{1}. \quad (\text{B4})$$

A degree- $q$  Majorana operator is a degree- $q$  monomial in the Majorana operators.

**Theorem B.1.** Let  $\mathcal{S}_q^n$  be the set of degree- $q$  Majorana operators on  $n$  fermionic modes with  $q$  even. Then

$$\left| \frac{\Delta(\mathcal{S}_q^n)}{\binom{n/2}{q/2} / \binom{n}{q}} - 1 \right| \leq \mathcal{O}(n^{-1}) \quad (\text{B5})$$

for all  $n$  sufficiently large, for each  $q$ .

See [Appendix B 5](#) for a proof of [Theorem B.1](#).

### 2. Commutation graph

Given a set  $\mathcal{S}$  of Pauli or Majorana operators, we can study their commutation structure by encoding it into a graph  $G$ , which we call the *commutation graph*.

**Definition B.3.** (Commutation graph.) The commutation graph  $G(\mathcal{S})$  of a set  $\mathcal{S}$  of Pauli or Majorana operators is defined as follows.

- The vertices of  $G(\mathcal{S})$  correspond to operators  $\mathbf{A} \in \mathcal{S}$ .
- We include an edge between any two vertices whose operators anticommute.

We now introduce a key graph property which reveals the anticommutativity of the operators  $\mathcal{S}$  through their commutation graph  $G(\mathcal{S})$ .

**Definition B.4.** (Lovász theta function.) Let  $G$  be a graph on  $m$  vertices. The Lovász theta function  $\vartheta(G)$  is defined by the following semidefinite program of dimension  $m$ . Let  $E$  denote the edges in the graph  $G$ , and  $\mathbb{J}$  the all-ones matrix.

$$\begin{aligned} \max \{ & \text{Tr}(\mathbb{J}\mathbf{X}) \ , \ \mathbf{X} \in \mathbb{R}^{m \times m} \\ \text{s.t. } & \mathbf{X} \succeq 0 \ , \ \text{Tr}(\mathbf{X}) = 1 \ , \ \mathbf{X}_{jl} = 0 \ \forall (j, l) \in E \}. \end{aligned} \quad (\text{B6})$$

It has dual

$$\begin{aligned} \min \{ & \lambda \in \mathbb{R} \\ \text{s.t. } & \exists \mathbf{Y} \in \mathbb{R}^{m \times m} \ , \ \mathbf{Y}_{jj} = 1 \ \forall j \ , \ \mathbf{Y}_{jl} = 0 \ \forall (j, l) \notin E \ , \ \lambda \mathbf{Y} \succeq \mathbb{J} \}. \end{aligned} \quad (\text{B7})$$

For any graph  $G$ , the following chain of inequalities is known:

$$I(G) \leq \vartheta(G) \leq \text{chrom}(\overline{G}), \quad (\text{B8})$$

where  $\overline{G}$  is the complement graph,  $\text{chrom}(\overline{G})$  is the chromatic number of  $\overline{G}$ , and  $I(G)$  is the independence number of  $G$ . For example, see [26]. Further, the commutation index  $\Delta(\mathcal{S})$  is bounded by the Lovász theta function of  $G(\mathcal{S})$ . All together, these bounds are expressed in the following lemma.

**Lemma B.1.** *Let  $\mathcal{S}$  be a set of Pauli operators*

$$\Delta(\mathcal{S}) \leq \frac{1}{|\mathcal{S}|} \vartheta(G(\mathcal{S})) \quad (\text{B9})$$

Further,

$$I(G(\mathcal{S})) \leq |\mathcal{S}| \cdot \Delta(\mathcal{S}) \leq \vartheta(G(\mathcal{S})) \leq \text{chrom}(\overline{G}(\mathcal{S})) \quad (\text{B10})$$

*Proof.* The inequality  $\Delta(\mathcal{S}) \leq \vartheta(G(\mathcal{S}))/|\mathcal{S}|$  has appeared previously in [39, Equation 5], [40, Proposition 3] and [19]. The inequalities  $I(G) \leq \vartheta(G) \leq \text{chrom}(\overline{G})$  always hold for any graph  $G$  [26]. Finally,  $\Delta(\mathcal{S}) \geq I(G(\mathcal{S}))/|\mathcal{S}|$  holds since we can choose  $\rho$  in the definition of  $\Delta(\mathcal{S})$  to be in the simultaneous eigenbasis of the independent set of operators.  $\square$

### 3. Proof of Proposition B.1

First we aim to show  $\mathbb{E}_{P \in \mathcal{P}_k^n} \langle \psi | P | \psi \rangle^2 = 3^{-k}$  for any product state of single-qubit states  $|\psi\rangle = |\psi_1\rangle \otimes \cdots \otimes |\psi_n\rangle$ . Denoting by  $\mathcal{P}_k^S$  the set of Paulis on subsystem  $S \subseteq [n]$ , we have

$$\mathbb{E}_{P \in \mathcal{P}_k^n} \langle \psi | P | \psi \rangle^2 = \mathbb{E}_{S \subseteq [n], |S|=k} \mathbb{E}_{P \in \mathcal{P}_k^S} \langle \psi | P | \psi \rangle^2 \quad (\text{B11})$$

By tracing out  $[n] \setminus S$ , it is sufficient to show

$$\mathbb{E}_{P \in \mathcal{P}_k^k} \langle \psi | P | \psi \rangle^2 = 3^{-k} \quad (\text{B12})$$

for any product state  $|\psi\rangle = |\psi_1\rangle \otimes \cdots \otimes |\psi_k\rangle$ . Since  $|\mathcal{P}_k^k| = 3^k$ , this is equivalent to

$$\sum_{P \in \mathcal{P}_k^k} \langle \psi | P | \psi \rangle^2 = 1 \quad (\text{B13})$$

But this holds since

$$\sum_{P \in \mathcal{P}_k^k} \langle \psi | P | \psi \rangle^2 = \prod_{j=1}^k \left( \sum_{P \in \{\sigma_x, \sigma_y, \sigma_z\}} \langle \psi_j | P | \psi_j \rangle^2 \right) = 1 \quad (\text{B14})$$

using that the single-qubit states  $|\psi_j\rangle$  are pure.

For the upper bound, we will invoke **Lemma B.1**. Recalling that  $G(\mathcal{P}_k^n)$  is the commutation graph of  $k$ -local Paulis, it suffices to show that  $\vartheta(G(\mathcal{P}_k^n)) \leq 3^{-k} \cdot |\mathcal{P}_k^n|$ . For this purpose, we import a fact from [26] using a proof technique similarly applied in [30]. A graph  $G$  is *vertex-symmetric* if for any two vertices  $u, v$ , there is an automorphism of  $G$  taking  $u$  to  $v$ .

**Fact B.1** ([26]). *If graph  $G$  is vertex-symmetric, then*

$$\vartheta(G) \cdot \vartheta(\overline{G}) = |G| \quad (\text{B15})$$

where  $\overline{G}$  is the complement graph and  $|G|$  denotes the number of vertices in  $G$ .

The commutation graph  $G(\mathcal{P}_k^n)$  of  $k$ -local Paulis is vertex-symmetric. Thus to establish the upper bound  $\vartheta(G(\mathcal{P}_k^n)) \leq 3^{-k} \cdot |\mathcal{P}_k^n|$ , it suffices to show  $\vartheta(\overline{G}(\mathcal{P}_k^n)) \geq 3^k$ . Using the independence number inequality of **Lemma B.1**, it suffices to find an independent set in  $\overline{G}(\mathcal{P}_k^n)$  of size at least  $3^k$ . This is equivalent to a *clique* in  $G(\mathcal{P}_k^n)$  of size at least  $3^k$ . In other words, we must exhibit a set of  $3^k$  mutually anticommuting  $k$ -local Paulis. This can be done using the ternary tree embedding of [64, 65]. Let  $2n+1 = 3^k$ . The ternary tree construction at depth  $k$  embeds  $2n+1$  mutually anticommuting operators into  $n$  qubits where each anticommuting operator has locality  $k$ .

### 4. Proof of Proposition B.2

For a fixed input state  $|\psi\rangle$ , we split the expectation by first sampling the choice of the subset  $S \subset [n]$  and then the particular choice of Paulis.

$$\mathbb{E}_{P \in \mathcal{P}_k^n} \langle \psi | P | \psi \rangle^2 = \mathbb{E}_{S \subseteq [n], |S|=k} \mathbb{E}_{P \in \mathcal{P}_k^S} \langle \psi | P | \psi \rangle^2. \quad (\text{B16})$$

Conditioned on each subset  $S$ , we compare the expectation over all  $3^k$ -many  $k$ -body Paulis with all  $4^k$  possible Pauli strings acting on the subset (i.e., including identities):

$$\mathbb{E}_{P \in \mathcal{P}_k^S} \langle \psi | P | \psi \rangle^2 = \frac{1}{3^k} \sum_{P \in \mathcal{P}_k^S} \langle \psi | P | \psi \rangle^2 \leq \frac{1}{3^k} \sum_{P \in (\mathbf{1}, \sigma_x, \sigma_y, \sigma_z)^{\otimes k}} \langle \psi | P | \psi \rangle^2 \quad (\text{Including strings with identities})$$

$$\leq \frac{1}{3^k} 4^k \langle \psi | \text{Tr}_S[|\psi\rangle \langle \psi|] \otimes \tau_S | \psi \rangle \quad (\text{B17})$$

$$\leq \frac{1}{3^k} 4^k \text{Tr}[\text{Tr}_S[|\psi\rangle \langle \psi|]^2 \otimes \tau_S^2] \quad (\text{B18})$$

$$\leq \frac{1}{3^k} 4^k \frac{1}{2^k} = \left(\frac{2}{3}\right)^k. \quad (\text{B19})$$

The second line uses that the average over all Pauli strings acts by partial-tracing out  $S$  and replacing with the maximally mixed state  $\tau_S$ :

$$\frac{1}{4^k} \sum_{P \in \mathcal{P}_k^S} P[\rho]P = \text{Tr}_S[\rho] \otimes \tau_S. \quad (\text{B20})$$

The last line uses that  $\text{Tr}[\tau_S^2] = 1/2^k$ , and that the purity of the reduced state is bounded by  $\text{Tr}[\rho^2] \leq 1$ . Take the expectation over subsets  $\mathbb{E}_{S \subseteq [n], |S|=k}$  to conclude the proof.

### 5. Proof of Theorem B.1

First we show the lower bound in Theorem B.1. We can find a set  $S \subseteq \mathcal{S}_q^n$  of mutually commuting degree- $q$  Majoranas of size  $\binom{n/2}{q/2}$  by taking  $(q/2)$ -wise products of  $\{i\gamma_1\gamma_2, i\gamma_3\gamma_4, \dots, i\gamma_{n-1}\gamma_n\}$ . Let  $\rho$  be the state which is maximally mixed within the simultaneous  $+1$ -eigenspace of the operators in  $S$ . Then  $\sum_{A \in \mathcal{S}_q^n} \langle \psi | A | \psi \rangle = |S| = \binom{n/2}{q/2}$  and  $\Delta(\mathcal{S}_q^n) \geq \binom{n/2}{q/2} / \binom{n}{q}$ . (Note the number of degree- $q$  monomials on  $n$  Majoranas is  $|\mathcal{S}_q^n| = \binom{n}{q}$ .) The remainder of this section is devoted to showing the upper bound via the Lovász theta function. We aim to establish the following theorem.

**Theorem B.2.** *Let  $\mathcal{S}_q^n$  be the set of degree- $q$  Majorana operators on  $n$  modes. Then*

$$\vartheta(G(\mathcal{S}_q^n)) \leq \binom{n/2}{q/2} + \mathcal{O}(e^{\mathcal{O}(q \log q) n^{q/2-1}}) \quad (\text{B21})$$

for all  $n$  sufficiently large, for each  $q$ .

Noting that  $|\mathcal{S}_q^n| = \binom{n}{q}$ , the upper bound in Theorem B.1 follows from combining Theorem B.2 and Lemma B.1. Thus it suffices to establish Theorem B.2.

After completing our work, we became aware of Ref. [43], in which they establish  $\vartheta(G(\mathcal{S}_q^n)) \leq \binom{n/2}{q/2} + c(q)n^{q/2-1}$  for some function  $c(q)$ . Theorem B.2 is stronger, since it specifies the asymptotic dependence  $c(q) = \mathcal{O}(e^{\mathcal{O}(q \log q)})$ . We give a self-contained proof of Theorem B.2.

The *Johnson association scheme*  $\mathcal{J}_d(m, r)$  is the graph whose vertices correspond to subsets  $S \subseteq [m]$  of size  $|S| = r$ , and  $(S, T)$  forms an edge if  $r - |S \cap T| = d$ . Write  $A_d^{m,r}$  for the adjacency matrix of the Johnson scheme  $\mathcal{J}_d(m, r)$ . The graph  $G(\mathcal{S}_q^n)$  has adjacency matrix  $A$  equal to

$$A = A_1^{n,q} + A_3^{n,q} + \dots + A_{q-1}^{n,q} \quad (\text{B22})$$

We are interested in the Lovász theta function of  $G(\mathcal{S}_q^n)$ . The following three results advertised in [19] reduce  $\vartheta(G(\mathcal{S}_q^n))$  to a linear program involving Hahn polynomials.

**Lemma B.2.** ([42, p. 48]) *The matrices  $A_0^{m,r}, \dots, A_r^{m,r}$  are simultaneously diagonalizable, with eigenvalues given by the dual Hahn polynomials:*

$$\text{spec}(A_d^{m,r}) = \{\tilde{H}_d^{m,r}(x) : x = 0, \dots, r\} \quad (\text{B23})$$

$$\tilde{H}_d^{m,r}(x) = \sum_{j=0}^d (-1)^{d-j} \binom{r-j}{d-j} \binom{r-x}{j} \binom{m-r+j-x}{j} \quad (\text{B24})$$

In particular, for all  $d > 0$  the all-1's vector is an eigenvector of  $A_d^{m,r}$  of multiplicity 1 with eigenvalue  $\tilde{H}_d^{m,r}(0) = \binom{m-r}{d} \binom{r}{d}$ .

**Lemma B.3.** *In the dual formulation of the Lovász theta function in Definition B.4 for the graph  $G(\mathcal{S}_q^n)$  it suffices to minimize over matrices  $Y$  whose entries  $Y(S, T)$  depend only on  $\text{dist}(S, T)$ . Thus we can write*

$$\vartheta(G(\mathcal{S}_q^n)) = \min\{\lambda : \exists a_1, a_3, \dots, a_{q-1} \text{ s.t. } \lambda(\mathbb{1} + a_1 A_1^{n,q} + a_3 A_3^{n,q} + \dots + a_{q-1} A_{q-1}^{n,q}) \succeq \mathbb{J}\}. \quad (\text{B25})$$

**Corollary B.1.**

$$\vartheta(G(\mathcal{S}_q^n)) = \min_{a_1, a_3, \dots, a_{q-1}} \left\{ \binom{n}{q} / (1 + p(0)) : p(1), \dots, p(r) \geq -1 \right. \\ \left. \text{where } p(x) = a_1 \tilde{H}_1^{n,q}(x) + a_3 \tilde{H}_3^{n,q}(x) + \dots + a_{q-1} \tilde{H}_{q-1}^{n,q}(x) \right\}. \quad (\text{B26})$$

Our strategy to prove Theorem B.2 is as follows. We will examine the linear program of Corollary B.1 in the large- $n$  limit. We will find that  $p(0)$  has the correct limit for large  $n$ , given by the following lemma.

**Lemma B.4.**  $p(0) = \frac{2^{q/2}(q/2)!}{(q)!} n^{q/2} - c(q) n^{q/2-1}$  for some  $c(q)$ .

This gives the correct large- $n$  limit for the Lovász theta function for constant  $q$ :

$$\binom{n}{q} / p(0) = \left( \frac{1}{(q)!} \cdot n^q + \mathcal{O}(n^{q-1}) \right) \cdot \left( \frac{2^{q/2}(q/2)!}{(q)!} n^{q/2} - \mathcal{O}(n^{q/2-1}) \right)^{-1} = \frac{1}{(q/2)!} (n/2)^{q/2} + \mathcal{O}(n^{q/2-1}) \quad (\text{B27})$$

Moreover,  $p(0)/n^{q/2}$  will be a low-degree polynomial in  $n^{-1}$ .

**Lemma B.5.**  $p(0)/n^{q/2}$  is a polynomial in  $n^{-1}$  of degree  $(q/2)(q/2 - 1)$ .

Finally, we will show a uniform bound on  $|p(0)/n^{q/2}|$ .

**Lemma B.6.**  $|p(0)/n^{q/2}| \leq e^{3q/2-3} q^{3q/2}/4$  for all  $n$ .

The polynomial method completes the argument by controlling the error for finite  $n$ . Markov's other inequality ([66, Theorem 5.1.8]) states the following (the particular bound on the  $1/n$ -values is worked out in, e.g., [67, 68]):

**Lemma B.7** (Markov's other inequality). *For any polynomial  $f(x)$  of degree  $p$ ,*

$$\sup_{x \in [-1, 1]} |f'(x)| \leq p^2 \sup_{x \in [-1, 1]} |f(x)|. \quad (\text{B28})$$

Consequently, there is an absolute constant  $c$  such that for any polynomial  $f$  of degree  $p$

$$|f(1/n) - f(0)| \leq \frac{cp^4}{n} \sup_{n' \geq 1} |f(1/n')| \quad \text{for each integer } n. \quad (\text{B29})$$

Combining Lemmas B.4, B.5, B.6 and B.7 completes the proof of Theorem B.2.

For constant  $r$ ,  $x = 1, \dots, r$  and large  $m$ , the leading order term in the Hahn polynomial is

$$\tilde{H}_d^{m,r}(x) = \begin{cases} \Theta(m^d) + \mathcal{O}(m^{d-1}) & d \leq r - x \\ (-1)^{d+x-r} \Theta(m^{r-x}) + \mathcal{O}(m^{r-x-1}) & d > r - x \end{cases} \quad (\text{B30})$$

Since it will be important in our application later, let us be more specific about the coefficients in the cases  $d = r - x - 1$  and  $d = r - x + 1$ . To leading order in  $m$  with  $r$  constant we have

$$\tilde{H}_{r-x-1}^{m,r}(x) = h^{(r)}(x) \cdot m^{r-x-1} + \mathcal{O}(m^{r-x-2}) \quad , \quad \tilde{H}_{r-x+1}^{m,r}(x) = -g^{(r)}(x) \cdot m^{r-x} + \mathcal{O}(m^{r-x-1}) \quad (\text{B31})$$

where

$$h^{(r)}(x) = \frac{r-x}{(r-x-1)!} \quad , \quad g^{(r)}(x) = \frac{x}{(r-x)!} \quad (B32)$$

Now let us examine the LP of Corollary B.1 in the large- $n$  limit. Our strategy is to sequentially go through  $x = q, \dots, 1$  and ensure that  $p(x) \geq -1$  for sufficiently large  $n$  for each  $x$ . For odd  $x$ , the equation  $p(x) \geq -1$  will automatically hold for sufficiently large  $n$ . Ensuring that  $p(x) \geq -1$  eventually for even  $x$  will require choosing  $a_{q-x+1}$  as a function of  $a_{q-x-1}$ . All coefficients  $a_1, a_3, \dots, a_{q-1}$  will be positive, and they will depend on  $n$  with scaling  $a_d = \Theta(n^{-(d-1)/2})$ . Henceforth let us assume these properties, and we will see that they can be satisfied.

Let us first consider  $p(q)$ . For all  $d$ ,  $\tilde{H}_d^{n,q}(q)$  are negative and in fact independent of  $n$ . The  $d = 1$  term is equal to  $-a_1 \cdot g^{(q)}(q)$ , and the terms  $d > 1$  are subleading order in  $n$  since they scale like  $a_d = \Theta(n^{-(d-1)/2})$  by assumption. Thus the constraint  $p(q) \geq -1$  is satisfied so long as

$$a_1 = \frac{C_1(n^{-1})}{g^{(q)}(q)} \quad (B33)$$

for some polynomial  $C_1$  of degree  $(q/2 - 1)$  with  $C_1(0) = 1$  and  $C_1(n^{-1}) \leq 1$  for all  $n$ .

For  $p(q-1)$ , all the Hahn polynomials appearing are positive to leading order in  $n$ , so  $p(q-1)$  is large and positive eventually. The same holds for any odd value of  $x$ .

Now consider  $p(q-2)$ . The  $d = 1$  term is  $a_1 \cdot h^{(q)}(q-2) \cdot n$  to leading order in  $n$ , and the  $d = 3$  term is  $-a_3 \cdot g^{(q)}(q-2) \cdot n^2$  to leading order. The terms  $d > 3$  are subleading order in  $n$  since they scale like  $a_d \cdot \Theta(n^2) = \Theta(n^{-(d+1)/2})$ . Thus we can satisfy the constraint  $p(q-2) \geq -1$  by picking

$$a_3 = C_3(n^{-1}) \cdot \frac{h^{(q)}(q-2)}{g^{(q)}(q-2)} \cdot n^{-1} \cdot a_1 \quad (B34)$$

$$= C_3(n^{-1}) \cdot C_1(n^{-1}) \cdot \frac{h^{(q)}(q-2)}{g^{(q)}(q-2)} \cdot \frac{1}{g^{(q)}(q)} \cdot n^{-1} \quad (B35)$$

for some polynomial  $C_3$  of degree  $(q/2 - 1)$  with  $C_3(0) = 1$  and  $C_3(n^{-1}) \leq 1$  for all  $n$ .

Continue like this for  $p(q-3), \dots, p(1)$ . For each even  $x$ , we will set

$$a_{q-x+1} = C_{q-x+1}(n^{-1}) \cdot \frac{h^{(q)}(x)}{g^{(q)}(x)} \cdot n^{-1} \cdot a_{q-x-1} \quad (B36)$$

$$= C_{q-x+1}(n^{-1}) \cdots C_1(n^{-1}) \cdot \frac{h^{(q)}(x) \cdot h^{(q)}(x+2) \cdots h^{(q)}(q-2)}{g^{(q)}(x) \cdot g^{(q)}(x+2) \cdots g^{(q)}(q-2)} \cdot \frac{1}{g^{(q)}(q)} \cdot n^{-q/2+(x/2)} \quad (B37)$$

For each even  $t$ ,  $C_{q-t+1}$  is a polynomial of degree  $(q/2 - 1)$  satisfying  $C_{q-t+1}(0) = 1$  and  $C_{q-t+1}(n^{-1}) \leq 1$  for all  $n$ . Notice that

$$\frac{h^{(q)}(t)}{g^{(q)}(t)} = \frac{(q-t)^2}{t} \quad (B38)$$

and  $g^{(q)}(q) = q$ , so defining

$$\hat{a}_{q-x+1} := \frac{(q-x)^2(q-x-2)^2 \dots 2^2}{(q-2)(q-4) \dots x} \cdot \frac{1}{q} \quad (B39)$$

we can write

$$a_{q-x+1} = C_{q-x+1}(n^{-1}) \cdots C_1(n^{-1}) \cdot \hat{a}_{q-x+1} \cdot n^{-q/2+(x/2)} \quad (B40)$$

Finally, let us look at  $p(0)$ . For large  $n$ , we have

$$p(0) = a_{q-1} \cdot h^{(q)}(0) \cdot n^{q-1} + \dots \quad (B41)$$

$$= C(n^{-1}) \cdot \frac{q}{(q-1)!} \cdot \hat{a}_{q-1} \cdot n^{q/2} - c(q)n^{q/2-1} \quad (B42)$$

where  $C = C_{q-1} \dots C_1$  is a polynomial of degree  $(q/2)(q/2 - 1)$  satisfying  $C(0) = 1$  and  $C(n^{-1}) \leq 1$  for all  $n$ , and  $c(q)$  is some function of  $q$ . (Note  $h^{(q)}(0) = q/(q-1)!$ .) The product in  $\hat{a}_{q-1}$  telescopes to give

$$\hat{a}_{q-1} = \frac{1}{q}(q-2)(q-4) \dots 2 \quad (B43)$$

so we get

$$p(0) = C(n^{-1}) \cdot \frac{1}{(q-1)(q-3)\dots 1} \cdot n^{q/2} - c(q)n^{q/2-1} \quad (\text{B44})$$

as promised. This establishes [Lemma B.4](#).

Let us now examine  $p(0)/n^{q/2}$ .

$$p(0)/n^{q/2} = \frac{1}{n^{q/2}} \left( a_1 \tilde{H}_1^{n,q}(0) + a_3 \tilde{H}_3^{n,q}(0) + \dots + a_{q-1} \tilde{H}_{q-1}^{n,q}(0) \right) \quad (\text{B45})$$

$$= \hat{a}_1 \binom{q}{1} \binom{n-q}{1} \cdot n^{-q/2} \cdot C_1(n^{-1}) + \hat{a}_3 \binom{q}{3} \binom{n-q}{3} \cdot n^{-q/2-1} \cdot (C_1 C_3)(n^{-1}) + \quad (\text{B46})$$

$$\dots + \hat{a}_{q-1} \binom{q}{q-1} \binom{n-q}{q-1} \cdot n^{-q+1} \cdot (C_1 \dots C_{q-1})(n^{-1}) \quad (\text{B47})$$

recalling  $\tilde{H}_d^{n,q}(0) = \binom{q}{d} \binom{n-q}{d}$ . Note the coefficients  $\hat{a}_d$  are independent of  $n$ . From this expression, we can readily see that  $p(0)/n^{q/2}$  is a polynomial in  $n$  of degree  $(q/2)(q/2-1)$ , establishing [Lemma B.5](#).

It remains to establish [Lemma B.6](#). For all  $d$ ,  $(C_1 \dots C_d)(n^{-1}) \leq 1$  and  $\hat{a}_d \leq \hat{a}_{q-1} = 2^{q/2-1}(q/2-1)!$ . Further, for all  $d$

$$\binom{q}{d} \binom{n-q}{d} \leq \left( \frac{2e^q}{d^2} \right)^d n^d \leq (2e^q)^{q-1} n^d \quad (\text{B48})$$

using the general bound  $\binom{m}{r} \leq (em/r)^r$ . Using these facts, we can bound

$$|p(0)/n^{q/2}| \leq 2^{q/2-1}(q/2-1)! \cdot (2e^q)^{q-1} \cdot (n^{-q/2+1} + n^{-q/2+2} + \dots + 1) \quad (\text{B49})$$

$$\leq e^{3q/2-3} q^{3q/2}/4 \quad (\text{B50})$$

using  $x! \leq x^{x+1}/e^{x-1}$  in the final step.

### 6. Numerics on Lovász theta function of local Majorana operators

In this section we present some numerics on the Lovász theta function of the commutation graph of degree- $q$  Majorana operators  $\vartheta(G(\mathcal{S}_q^n))$ .

| $n$ | $\vartheta(G(\mathcal{S}_q^n))$ |       |        |        |        | $\binom{n/2}{q/2}$ |       |       |       |        |
|-----|---------------------------------|-------|--------|--------|--------|--------------------|-------|-------|-------|--------|
|     | $q=2$                           | $q=4$ | $q=6$  | $q=8$  | $q=10$ | $q=2$              | $q=4$ | $q=6$ | $q=8$ | $q=10$ |
| 2   | 1                               |       |        |        |        | 1                  |       |       |       |        |
| 4   | 2                               | 1     |        |        |        | 2                  | 1     |       |       |        |
| 6   | 3                               | 3     | 1      |        |        | 3                  | 3     | 1     |       |        |
| 8   | 4                               | 14    | 4      | 1      |        | 4                  | 6     | 4     | 1     |        |
| 10  | 5                               | 14.57 | 14.57  | 5      | 1      | 5                  | 10    | 10    | 5     | 1      |
| 12  | 6                               | 15    | 52     | 15     | 6      | 6                  | 15    | 20    | 15    | 6      |
| 14  | 7                               | 21    | 57.34  | 57.34  | 21     | 7                  | 21    | 35    | 35    | 21     |
| 16  | 8                               | 28    | 64     | 198    | 64     | 8                  | 28    | 56    | 70    | 56     |
| 18  | 9                               | 36    | 100.13 | 218.34 | 218.34 | 9                  | 36    | 84    | 126   | 126    |
| 20  | 10                              | 45    | 153.11 | 251.22 | 787.17 | 10                 | 45    | 120   | 210   | 252    |
| 22  | 11                              | 55    | 195.13 | 429.91 | 885.15 | 11                 | 55    | 165   | 330   | 462    |
| 24  | 12                              | 66    | 236.42 | 759    | 982.84 | 12                 | 66    | 220   | 495   | 792    |
| 26  | 13                              | 78    | 286    | 990.80 | 1757.0 | 13                 | 78    | 286   | 715   | 1287   |
| 28  | 14                              | 91    | 364    | 1217.2 | 3260.2 | 14                 | 91    | 364   | 1001  | 2002   |
| 30  | 15                              | 105   | 455    | 1444.2 | 4643.9 | 15                 | 105   | 455   | 1365  | 3003   |
| 32  | 16                              | 120   | 560    | 1820.0 | 6040.7 | 16                 | 120   | 560   | 1820  | 4368   |
| 34  | 17                              | 136   | 680    | 2423.3 | 7240.0 | 17                 | 136   | 680   | 2380  | 6188   |
| 36  | 18                              | 153   | 816    | 3327.1 | 9269.4 | 18                 | 153   | 816   | 3060  | 8568   |
| 38  | 19                              | 171   | 969    | 4512.8 | 12552  | 19                 | 171   | 969   | 3876  | 11628  |
| 40  | 20                              | 190   | 1140   | 6022.1 | 17230  | 20                 | 190   | 1140  | 4845  | 15504  |

Table IV. Numerical comparison of the Lovász theta function  $\vartheta(G(\mathcal{S}_q^n))$  versus  $\binom{n/2}{q/2}$ . They are exactly equal for very small values of  $n$ , and also appear to be exactly equal for sufficiently large values of  $n$  for each  $q$ . For example at  $q=4$ , which corresponds to the standard SYK-4 model, it appears that  $\vartheta(G(\mathcal{S}_4^n)) = \binom{n/2}{2}$  for all even values of  $n$  apart from  $n=8$  and  $n=10$ .

![Figure 1: Log-log plot titled 'Comparison of Lovász Theta and Binomial'. The x-axis is labeled 'n' and ranges from 10^1 to 10^3. The y-axis ranges from 10^1 to 10^7. The plot shows two sets of curves for different values of q (2, 4, 6, 8, 10, 12). For each q, there is a solid line representing 'Lovász Theta' and a dashed line representing 'Binomial'. The curves for q=2 are the lowest, and the curves for q=12 are the highest. The curves for q=10 and q=12 show some initial fluctuations for small n before following a linear trend on the log-log scale, indicating a power-law relationship. The legend on the right side of the plot identifies the colors for each q value and the line styles for 'Quantity', 'Lovász Theta', and 'Binomial'.](eaae122ace5c0d761133c6ce971a6ffd_img.jpg)

Figure 1: Log-log plot titled 'Comparison of Lovász Theta and Binomial'. The x-axis is labeled 'n' and ranges from 10^1 to 10^3. The y-axis ranges from 10^1 to 10^7. The plot shows two sets of curves for different values of q (2, 4, 6, 8, 10, 12). For each q, there is a solid line representing 'Lovász Theta' and a dashed line representing 'Binomial'. The curves for q=2 are the lowest, and the curves for q=12 are the highest. The curves for q=10 and q=12 show some initial fluctuations for small n before following a linear trend on the log-log scale, indicating a power-law relationship. The legend on the right side of the plot identifies the colors for each q value and the line styles for 'Quantity', 'Lovász Theta', and 'Binomial'.

Figure 1. Log-log plot of  $\vartheta(G(\mathcal{S}_q^n))$  versus  $\binom{n}{q/2}$ .  $\vartheta(G(\mathcal{S}_q^n))$  fluctuates for small  $n$ , but for sufficiently large  $n$  it behaves the same as  $\binom{n}{q/2}$ .

### 7. Alternative definition of commutation index

**Lemma B.8.** *When the left and right test vectors are different, we still have*

$$\Delta(\mathcal{S}) \leq \sup_{\|u\|=\|v\|=1} \mathbb{E}_{A \in \mathcal{S}} |\langle u | A | v \rangle|^2 = \sup_O \frac{\mathbb{E}_{A \in \mathcal{S}} |\text{Tr}[AO]|^2}{\|O\|_1^2} \leq 16\Delta(\mathcal{S}). \quad (\text{B51})$$

*Proof.* The first inequality is trivial. The middle equality follows from the first by taking the singular value decomposition of  $O$ . To prove the final inequality, for every  $u, v$ , define polarizations

$$|s\rangle := \frac{|u\rangle + s|v\rangle}{2} \quad \text{where } s = \pm 1, \pm i. \quad (\text{B52})$$

Then, for each  $A$ ,

$$|\langle u | A | v \rangle|^2 = |4\mathbb{E}_s s \langle s | A | s \rangle|^2 \quad (\text{B53})$$

$$\leq 16\mathbb{E}_s |\langle s | A | s \rangle|^2. \quad (\text{B54})$$

Hence,

$$\sup_{\|u\|=\|v\|=1} \mathbb{E}_{A \in \mathcal{S}} |\langle u | A | v \rangle|^2 \leq 16 \sup_{\|u\|=\|v\|=1} \mathbb{E}_{A \in \mathcal{S}} \mathbb{E}_s |\langle s | A | s \rangle|^2 \quad (\text{B55})$$

$$= 16 \sup_{\|u\|=\|v\|=1} \mathbb{E}_s \mathbb{E}_{A \in \mathcal{S}} |\langle s | A | s \rangle|^2 \quad (\text{B56})$$

$$\leq 16\Delta(\mathcal{S}). \quad (\text{B57})$$

□

## Appendix C: Annealed approximation and concentration results

Consider the model

$$H = \frac{1}{\sqrt{m}} \sum_{i=1}^m g_i A_i, \quad (\text{C1})$$

where  $g_i \sim_{i.i.d.} \mathcal{N}(0, 1)$  are standard independent Gaussians and  $\mathbf{A}_i$  are deterministic matrices. The *Gibbs state*  $\rho_\beta$  at inverse temperature  $\beta$  is defined by

$$\rho_\beta = \frac{e^{-\beta\sqrt{n}\mathbf{H}}}{Z_\beta} \quad , \quad Z_\beta = \text{Tr} \left( e^{-\beta\sqrt{n}\mathbf{H}} \right), \quad (\text{C2})$$

where  $Z_\beta$  is called the *partition function* at inverse temperature  $\beta$ . The factor  $\sqrt{n}$  ensures that the free energy is extensive and scales proportionally to  $n$ .

In this section we show that the commutation index of the terms  $\mathbf{A}_i$  has an important effect on the concentration properties of the random model  $\mathbf{H}$ . Denote the commutation index by

$$\Delta := \Delta(\{\mathbf{A}_i\}_{i=1}^m). \quad (\text{C3})$$

Recall that this quantity characterizes the variance of the energy with respect to a fixed state  $\rho$ :

$$\sup_{\rho} \mathbb{E}_{\mathbf{H}} |\text{Tr}(\mathbf{H}\rho)|^2 = \sup_{\rho} \frac{1}{m} \sum_{i=1}^m (\text{Tr}(\mathbf{A}_i\rho))^2 = \Delta. \quad (\text{C4})$$

The value of  $\Delta$  has implications for relations between the normalized quenched and annealed free energies:

$$\underbrace{\frac{1}{n} \mathbb{E} \ln Z_\beta}_{\text{quenched}} \quad \text{vs.} \quad \underbrace{\frac{1}{n} \ln \mathbb{E} Z_\beta}_{\text{annealed}}. \quad (\text{C5})$$

The quenched free energy is physical but hard to calculate while the annealed free energy is much easier to calculate but nonphysical. The first result is that this variance quantity controls the difference between the two.

**Theorem C.1** (Quenched and annealed free energy).

$$n^{-1} \mathbb{E} \ln Z_\beta \leq n^{-1} \ln \mathbb{E} Z_\beta \leq n^{-1} \mathbb{E} \ln Z_\beta + 4\beta^2 \Delta. \quad (\text{C6})$$

The first inequality always holds, and the non-trivial part is the second inequality. Therefore, a small variance  $\Delta \ll \beta^{-2}$  means that the annealed free energy well-approximates the quenched free energy, which indicates the absence of spin glass order [13]. The next three results give concentration of expectation values, energy, and two-point correlators of the thermal state of  $\mathbf{H}$ . Two-point correlators are of special interest in the study of the SYK model [13, 58, 69, 70]. Concentration results for Lipschitz bounded functions of the spectrum of the SYK model have also been established in [71].

**Theorem C.2** (Concentration of expectation values). *For any fixed bounded Hermitian operator  $\mathbf{X}$ ,*

$$\mathbb{P}(|\text{Tr}(\mathbf{X}\rho_\beta) - \mathbb{E}\text{Tr}(\mathbf{X}\rho_\beta)| \geq t) \leq 2e^{-t^2/(18\beta^2\|\mathbf{X}\|^2\Delta)}. \quad (\text{C7})$$

**Theorem C.3** (Concentration of energy).

$$\mathbb{P}(|\text{Tr}(\mathbf{H}\rho_\beta) - \mathbb{E}\text{Tr}(\mathbf{H}\rho_\beta)| \geq t) \leq 4 \exp \left( -\frac{1}{2\Delta} \left( \sqrt{\frac{t^2}{12\beta^2 n} + \alpha^2} - \alpha \right) \right) \quad (\text{C8})$$

where  $\alpha = \frac{1}{2}(1/(4\beta^2 n) + \mathbb{E}[\lambda_{\max}(\mathbf{H})]^2)$ .

**Theorem C.4** (Concentration of two-point correlators). *For any fixed bounded operators  $\mathbf{X}$  and  $\mathbf{Y}$ , denoting*

$$\mathbf{Y}(\tau) = \exp(i\sqrt{n}\mathbf{H}\tau) \mathbf{Y} \exp(-i\sqrt{n}\mathbf{H}\tau) \quad (\text{C9})$$

for any  $\tau \in \mathbb{R}$ , we have:

$$\mathbb{P}\left(\frac{1}{2} |\text{Tr}(\mathbf{XY}(\tau)\rho_\beta) - \mathbb{E}\text{Tr}(\mathbf{XY}(\tau)\rho_\beta) \pm h.c.| \geq t\right) \leq 2e^{-t^2/(6n(5\beta^2+16\tau^2)\|\mathbf{X}\|^2\|\mathbf{Y}\|^2\Delta)}. \quad (\text{C10})$$

Instantiating for example Theorems C.1, C.2, C.3, C.4 with the upper bound on the commutation index of Majoranas in Theorem B.1 yields the following results for the SYK model.

**Corollary C.1.** (SYK model is annealed.) *For the SYK model  $\mathbf{H}_q^{\text{SYK}}$  where  $q$  is even,*

$$\frac{1}{n} \mathbb{E} \ln Z_\beta \leq \frac{1}{n} \ln \mathbb{E} Z_\beta \leq \frac{1}{n} \mathbb{E} \ln Z_\beta + \mathcal{O}_q(\beta^2 n^{-q/2}), \quad (\text{C11})$$

$$\mathbb{P}(|\text{Tr}(\mathbf{X}\rho_\beta) - \mathbb{E}\text{Tr}(\mathbf{X}\rho_\beta)| \geq t) \leq 2e^{-\Omega_q(\beta^{-2}n^{q/2-1}t^2)}, \quad (\text{C12})$$

$$\mathbb{P}(|\text{Tr}(\mathbf{H}_q^{\text{SYK}}\rho_\beta) - \mathbb{E}\text{Tr}(\mathbf{H}_q^{\text{SYK}}\rho_\beta)| \geq t) \leq \begin{cases} 4e^{-\Omega_q(\beta^{-1}n^{q/2-1/2}t)} & t = \Omega(1 + \beta n) \\ 4e^{-\Omega_q(\min(1, \beta^{-2}n^{-2})n^{q/2}t^2)} & \text{otherwise} \end{cases}, \quad (\text{C13})$$

$$\mathbb{P}\left(\frac{1}{2} |\text{Tr}(\mathbf{XY}(\tau)\rho_\beta) - \mathbb{E}\text{Tr}(\mathbf{XY}(\tau)\rho_\beta) \pm h.c.| \geq t\right) \leq 2e^{-\Omega_q(\min(\beta^{-2}, \tau^{-2})n^{q/2-1}t^2)}. \quad (\text{C14})$$

$\mathbf{X}$  and  $\mathbf{Y}$  are any fixed bounded operators.

Importantly, the above result shows that for the standard SYK model with  $q = 4$ , the quenched free energy in the limit of  $n \rightarrow \infty$  always equal its annealed approximation for physical temperatures where  $\beta = \Theta(\sqrt{n})$ . This stands in stark contrast with spin glasses where a transition occurs for some critical temperature  $\beta_p$  into a clustered or ‘glassy’ phase (see [Appendix A](#)).

The remainder of this appendix is concerned with establishing [Theorems C.1](#), [C.2](#), [C.3](#), [C.4](#) for concentration of various observables and free energies.

### 1. Preliminaries

Let us first state a useful fact about Lipschitz functions of Gaussian variables.

**Fact C.1** (Gaussian concentration of Lipschitz functions, Theorem 2.26 of [72]). *Let  $\mathbf{g} = (g_1, \dots, g_m)$  be i.i.d. standard Gaussian variables, and  $f : \mathbb{R}^m \rightarrow \mathbb{R}$   $L$ -Lipschitz. Then for any  $t \geq 0$ :*

$$\mathbb{P}(|f(\mathbf{g}) - \mathbb{E}f(\mathbf{g})| \geq t) \leq 2e^{-t^2/(2L^2)} \quad (\text{C15})$$

We also state a useful fact on the concentration of the operator norm of random matrices of the form of  $\mathbf{H}$ .

**Fact C.2** (Concentration of the maximal eigenvalue [73, Corollary 4.14]). *Let  $\lambda_{\max}(\mathbf{H})$  be the maximal eigenvalue of  $\mathbf{H}$ . We have:*

$$\Pr(\lambda_{\max}(\mathbf{H}) - \mathbb{E}\lambda_{\max}(\mathbf{H}) \geq t) \leq \exp\left(-\frac{t^2}{2\Delta}\right). \quad (\text{C16})$$

In the course of proving our results we will also use an equivalent formulation of [Fact C.1](#) that follows from its sub-Gaussianity [74–76].

**Lemma C.1** (Sub-Gaussian MGF bound, Lemma 1.5 of Ref. [74]). *Given a random variable  $X$  with sub-Gaussian concentration bound*

$$\mathbb{P}(|X - \mathbb{E}X| \geq t) \leq 2e^{-t^2/(2\sigma^2)}, \quad (\text{C17})$$

it holds that

$$\mathbb{E}[\exp(t(X - \mathbb{E}X))] \leq \exp(4\sigma^2 t^2). \quad (\text{C18})$$

### 2. Proof of Theorem C.1

We directly compute the derivatives of  $\ln Z_\beta$  with respect to each Gaussian  $g_i$

$$\begin{aligned} \partial_{g_i} \ln Z_\beta &= \frac{1}{Z_\beta} \text{Tr}[\partial_{g_i} e^{\beta\sqrt{n}\mathbf{H}}] \\ &= \frac{1}{Z_\beta} \text{Tr} \left[ \beta \sqrt{\frac{n}{m}} \int_0^1 e^{\beta\sqrt{n}\mathbf{H}(1-s)} A_i e^{\beta\sqrt{n}\mathbf{H}s} ds \right] && (\text{Derivative of matrix exponential [77]}) \\ &= \beta \sqrt{\frac{n}{m}} \text{Tr}[A_i \rho_\beta]. && (\text{Cyclic property of trace}) \end{aligned} \quad (\text{C19})$$

Therefore, the Lipschitz constant  $L$  of  $\ln Z_\beta$  with respect to the disorder has the gradient bound:

$$L^2 \leq \frac{\beta^2 n}{m} \sum_{i=1}^m \text{Tr}[A_i \rho_\beta]^2 \leq \beta^2 n \Delta. \quad (\text{C20})$$

Now we can bound

$$\frac{\mathbb{E}[Z_\beta]}{\exp(\mathbb{E}[\ln Z_\beta])} = \mathbb{E}[\exp(\ln Z_\beta - \mathbb{E}[\ln Z_\beta])] \leq \exp(4\beta^2 n \Delta). \quad (\text{C21})$$

The inequality uses [Fact C.1](#) and [Lemma C.1](#) with  $t = 1$ . Taking logarithms and rearrange to obtain

$$\frac{1}{n} \log \mathbb{E}[Z_\beta] \leq \frac{1}{n} \mathbb{E}[\ln Z_\beta] + 4\beta^2 \Delta, \quad (\text{C22})$$

as stated.

### 3. Proof of Theorem C.2

The result once again follows from a Lipschitz bound. We use the well-known expression for the derivative of a matrix exponential [77]:

$$\partial_{g_i} \exp(\mathbf{H}) = \int_0^1 \exp(t\mathbf{H}) (\partial_{g_i} \mathbf{H}) \exp((1-t)\mathbf{H}) dt. \quad (\text{C23})$$

From the chain rule we then have:

$$\partial_{g_i} \text{Tr}(\rho_\beta \mathbf{X}) = \frac{\beta\sqrt{n}}{\sqrt{m}} \left( Z_\beta^{-1} \text{Tr} \left( \mathbf{X} \int_0^1 \exp(t\beta\sqrt{n}\mathbf{H}) \mathbf{A}_j \exp((1-t)\beta\sqrt{n}\mathbf{H}) dt \right) - \text{Tr}(\mathbf{X}\rho_\beta) \text{Tr}(\mathbf{A}_j\rho_\beta) \right). \quad (\text{C24})$$

Consider now the operator:

$$\sigma_\beta \equiv Z_\beta^{-1} \int_0^1 \exp(t\beta\sqrt{n}\mathbf{H}) \mathbf{X} \exp((1-t)\beta\sqrt{n}\mathbf{H}) dt. \quad (\text{C25})$$

We can check that  $\sigma_\beta$  has trace norm bounded by  $\|\mathbf{X}\|$ . Denoting by  $\Sigma_i(\cdot)$  the  $i$ th singular value of  $\cdot$  in nonincreasing order, we have by the majorization inequality [78]:

$$\sum_i \Sigma_i(\mathbf{AB}) \leq \sum_i \Sigma_i(\mathbf{A}) \Sigma_i(\mathbf{B}) \quad (\text{C26})$$

that

$$\begin{aligned} \sum_i \Sigma_i(\exp(t\beta\sqrt{n}\mathbf{H}) \mathbf{X} \exp((1-t)\beta\sqrt{n}\mathbf{H})) &\leq \|\mathbf{X}\| \sum_i \Sigma_i(\exp(t\beta\sqrt{n}\mathbf{H})) \Sigma_i(\exp((1-t)\beta\sqrt{n}\mathbf{H})) \\ &\leq \|\mathbf{X}\| Z_\beta. \end{aligned} \quad (\text{C27})$$

This implies that  $\sigma_\beta$  has trace norm bounded by  $\|\mathbf{X}\|$ . However,  $\sigma_\beta$  is Hermitian but not necessarily positive semidefinite. We instead consider  $\sigma_\beta$  as the difference of two positive semidefinite matrices:

$$\sigma_\beta = \sigma_\beta^+ - \sigma_\beta^-, \quad (\text{C28})$$

where each of  $\sigma_\beta^\pm$  is a (unnormalized) quantum state. By the cyclic property of the trace we can then write:

$$\partial_{g_i} \text{Tr}(\rho_\beta \mathbf{X}) = \frac{\beta\sqrt{n}}{\sqrt{m}} \left( \text{Tr}(\sigma_\beta^+ \mathbf{A}_j) - \text{Tr}(\sigma_\beta^- \mathbf{A}_j) - \text{Tr}(\mathbf{X}\rho_\beta) \text{Tr}(\mathbf{A}_j\rho_\beta) \right). \quad (\text{C29})$$

We thus have:

$$\|\nabla_{\mathbf{g}} \text{Tr}(\rho_\beta \mathbf{X})\|_2^2 = \frac{\beta^2 n}{m} \sum_j \left( \text{Tr}(\sigma_\beta^+ \mathbf{A}_j) - \text{Tr}(\sigma_\beta^- \mathbf{A}_j) - \text{Tr}(\mathbf{X}\rho_\beta) \text{Tr}(\mathbf{A}_j\rho_\beta) \right)^2 \quad (\text{C30})$$

$$\leq \frac{3\beta^2 n}{m} \sum_j \left( (\text{Tr} \sigma_\beta^+ \mathbf{A}_j)^2 + (\text{Tr} \sigma_\beta^- \mathbf{A}_j)^2 + (\text{Tr} \mathbf{X} \rho_\beta)^2 (\text{Tr} \mathbf{A}_j \rho_\beta)^2 \right) \quad (\text{C31})$$

$$\leq 9\beta^2 n \|\mathbf{X}\|^2 \Delta. \quad (\text{C32})$$

The result then follows from Fact C.1.

### 4. Proof of Theorem C.3

We would like an analog of Eq. (C29) where the observable is  $\mathbf{H}$ . Notice this is  $g_i$ -dependent and commutes with  $\rho_\beta$ . We get:

$$\partial_{g_i} \text{Tr}(\rho_\beta \mathbf{H}) = \frac{1}{\sqrt{m}} \text{Tr}(\rho_\beta \mathbf{A}_j) + \frac{\beta\sqrt{n}}{\sqrt{m}} (\text{Tr}(\rho_\beta \mathbf{H} \mathbf{A}_j) - \text{Tr}(\mathbf{H} \rho_\beta) \text{Tr}(\mathbf{A}_j \rho_\beta)). \quad (\text{C33})$$

Let  $\lambda_{\max}(\mathbf{H})$  denote the maximal eigenenergy of  $\mathbf{H}$ , and let  $\mathcal{G}_s$  be the set of coefficients  $\mathbf{g}$  where  $\lambda_{\max}(\mathbf{H}) \leq s + \mathbb{E}\lambda_{\max}(\mathbf{H})$ . For  $\mathbf{g} \in \mathcal{G}_s$ :

$$\|\nabla_{\mathbf{g}} \text{Tr}(\rho_{\beta} \mathbf{H})\|_2^2 = \frac{1}{m} \sum_j (\text{Tr}(\rho_{\beta} \mathbf{A}_j) + \beta\sqrt{n} \text{Tr}(\rho_{\beta} \mathbf{H} \mathbf{A}_j) - \beta\sqrt{n} \text{Tr}(\mathbf{H} \rho_{\beta}) \text{Tr}(\mathbf{A}_j \rho_{\beta}))^2 \quad (\text{C34})$$

$$\leq \frac{3}{m} \sum_j \left( (\text{Tr} \rho_{\beta} \mathbf{A}_j)^2 + \beta^2 n (\text{Tr} \rho_{\beta} \mathbf{H} \mathbf{A}_j)^2 + \beta^2 n (\text{Tr} \rho_{\beta} \mathbf{H})^2 (\text{Tr} \rho_{\beta} \mathbf{A}_j)^2 \right) \quad (\text{C35})$$

$$\leq 3\Delta(1 + 2\beta^2 n \|\mathbf{H}\|^2) \quad (\text{C36})$$

$$\leq 3\Delta(1 + 2\beta^2 n (s + \mathbb{E}\lambda_{\max}(\mathbf{H}))^2). \quad (\text{C37})$$

By [Fact C.2](#),  $\mathbb{P}[\mathbf{g} \notin \mathcal{G}_s] \leq 2 \exp\left(-\frac{s^2}{2\Delta}\right)$ . Furthermore,  $\mathcal{G}_s$  is a convex set, and thus there exists a function  $\hat{\text{Tr}}(\rho_{\beta} \mathbf{H})$  that agrees with  $\text{Tr} \rho_{\beta} \mathbf{H}$  on  $\mathcal{G}_s$  yet has the same Lipschitz bound of Eq. (C34) on the full domain. We thus calculate:

$$\mathbb{P}[|\text{Tr}(\mathbf{H} \rho_{\beta}) - \mathbb{E}[\text{Tr}(\mathbf{H} \rho_{\beta})]| \geq t] \leq \inf_s \mathbb{P}[|\text{Tr}(\mathbf{H} \rho_{\beta}) - \mathbb{E}[\text{Tr}(\mathbf{H} \rho_{\beta})]| \geq t \wedge \mathbf{g} \in \mathcal{G}_s] + 2 \exp\left(-\frac{s^2}{2\Delta}\right) \quad (\text{C38})$$

$$= \inf_s \mathbb{P}[|\hat{\text{Tr}}(\mathbf{H} \rho_{\beta}) - \mathbb{E}[\hat{\text{Tr}}(\mathbf{H} \rho_{\beta})]| \geq t \wedge \mathbf{g} \in \mathcal{G}_s] + 2 \exp\left(-\frac{s^2}{2\Delta}\right) \quad (\text{C39})$$

$$\leq \inf_s \mathbb{P}[|\hat{\text{Tr}}(\mathbf{H} \rho_{\beta}) - \mathbb{E}[\hat{\text{Tr}}(\mathbf{H} \rho_{\beta})]| \geq t] + 2 \exp\left(-\frac{s^2}{2\Delta}\right) \quad (\text{C40})$$

$$\leq \inf_s 2 \exp\left(-\frac{t^2}{6\Delta(1 + 2\beta^2 n (s + \mathbb{E}[\lambda_{\max}(\mathbf{H})])^2)}\right) + 2 \exp\left(-\frac{s^2}{2\Delta}\right) \quad (\text{C41})$$

$$\leq \inf_s 2 \exp\left(-\frac{t^2}{6\Delta(1 + 4\beta^2 n (s^2 + \mathbb{E}[\lambda_{\max}(\mathbf{H})]^2))}\right) + 2 \exp\left(-\frac{s^2}{2\Delta}\right) \quad (\text{C42})$$

$$\leq \inf_s 2 \exp\left(-\frac{t^2}{24\beta^2 \Delta n (s^2 + 1/(4\beta^2 n) + \mathbb{E}[\lambda_{\max}(\mathbf{H})]^2)}\right) + 2 \exp\left(-\frac{s^2}{2\Delta}\right). \quad (\text{C43})$$

Setting  $s^2 = \sqrt{t^2/(12\beta^2 n)} + \alpha^2 - \alpha$  where  $\alpha = \frac{1}{2}(1/(4\beta^2 n) + \mathbb{E}[\lambda_{\max}(\mathbf{H})]^2)$  gives the desired result.

### 5. Proof of [Theorem C.4](#)

Completely analogously to the proof of [Theorem C.2](#) we have:

$$\partial_{g_i} \text{Tr}(\rho_{\beta} \mathbf{X} \mathbf{Y}(\tau)) = \frac{\beta\sqrt{n}}{\sqrt{m}} (\text{Tr}(\sigma_{\beta} \mathbf{A}_j) - \text{Tr}(\mathbf{X} \mathbf{Y}(\tau) \rho_{\beta}) \text{Tr}(\mathbf{A}_j \rho_{\beta})) + \text{Tr}(\rho_{\beta} \mathbf{X} \partial_{g_i} \mathbf{Y}(\tau)), \quad (\text{C44})$$

where:

$$\sigma_{\beta} = Z_{\beta}^{-1} \int_0^1 \exp(t\beta\sqrt{n}\mathbf{H}) \mathbf{X} \mathbf{Y}(\tau) \exp((1-t)\beta\sqrt{n}\mathbf{H}) dt. \quad (\text{C45})$$

We now focus on the final term of Eq. (C44). We have:

$$\partial_{g_i} \mathbf{Y}(\tau) = \frac{i\tau\sqrt{n}}{\sqrt{m}} \left( \int_0^1 \exp(i\tau\sqrt{n}t\mathbf{H}) \mathbf{A}_j \exp(i\tau\sqrt{n}(1-t)\mathbf{H}) dt \right) \mathbf{Y} \exp(-i\tau\sqrt{n}\mathbf{H}) + \text{h.c.} = \frac{i\tau\sqrt{n}}{\sqrt{m}} [\tilde{\mathbf{A}}_{j|\tau}, \mathbf{Y}(\tau)], \quad (\text{C46})$$

where  $\tilde{\mathbf{A}}_{j|\tau}$  is the Hermitian, time-averaged operator:

$$\tilde{\mathbf{A}}_{j|\tau} = \int_0^1 \exp(i\tau\sqrt{n}t\mathbf{H}) \mathbf{A}_j \exp(-i\tau\sqrt{n}t\mathbf{H}) dt = \frac{1}{\tau} \int_0^{\tau} \exp(it\sqrt{n}\mathbf{H}) \mathbf{A}_j \exp(-it\sqrt{n}\mathbf{H}) dt. \quad (\text{C47})$$

We have:

$$\frac{1}{4} \left\| \nabla_{\mathbf{g}} \operatorname{Tr} (\rho_{\beta} \mathbf{X} \mathbf{Y} (\tau)) \pm \text{h.c.} \right\|_2^2 \quad (\text{C48})$$

$$= \frac{n}{4m} \sum_j \left| \beta \operatorname{Tr} (\sigma_{\beta} \mathbf{A}_j) - \beta \operatorname{Tr} (\mathbf{X} \mathbf{Y} (\tau) \rho_{\beta}) \operatorname{Tr} (\mathbf{A}_j \rho_{\beta}) + i\tau \operatorname{Tr} \rho_{\beta} \mathbf{X} [\tilde{\mathbf{A}}_{j|\tau}, \mathbf{Y} (\tau)] \pm \text{h.c.} \right|^2 \quad (\text{C49})$$

$$\leq \frac{3n}{4m} \sum_j \left( |\beta \operatorname{Tr} (\sigma_{\beta} \mathbf{A}_j) \pm \text{h.c.}|^2 + |\beta \operatorname{Tr} (\mathbf{X} \mathbf{Y} (\tau) \rho_{\beta}) \operatorname{Tr} (\mathbf{A}_j \rho_{\beta}) \pm \text{h.c.}|^2 + \left| \tau \operatorname{Tr} \left( \rho_{\beta} \mathbf{X} [\tilde{\mathbf{A}}_{j|\tau}, \mathbf{Y} (\tau)] \right) \pm \text{h.c.} \right|^2 \right). \quad (\text{C50})$$

To proceed, consider (for instance):

$$\operatorname{Tr} \left( (\mathbf{Y} (\tau) \rho_{\beta} \mathbf{X} + \text{h.c.}) \tilde{\mathbf{A}}_{j|\tau} \right) \equiv \operatorname{Tr} (\mu_{\beta,\tau} \mathbf{A}_j). \quad (\text{C51})$$

$\mu_{\beta,\tau}$  is Hermitian by definition and has trace norm bounded by  $2 \|\mathbf{X}\| \|\mathbf{Y}\|$  by the matrix Hölder inequality (or alternatively, an analog of the singular value majorization argument used in the proof of [Theorem C.2](#)). Once again we can consider  $\mu_{\beta,\tau}$  as the difference of two positive semidefinite matrices:

$$\mu_{\beta,\tau} = \mu_{\beta,\tau}^+ - \mu_{\beta,\tau}^- \quad (\text{C52})$$

Doing this for all terms yields:

$$\frac{1}{4} \left\| \nabla_{\mathbf{g}} \operatorname{Tr} (\rho_{\beta} \mathbf{X} \mathbf{Y} (\tau)) \pm \text{h.c.} \right\|_2^2 \quad (\text{C53})$$

$$\leq \frac{3n}{4m} \sum_j \left( |\beta \operatorname{Tr} (\sigma_{\beta} \mathbf{A}_j) \pm \text{h.c.}|^2 + |\beta \operatorname{Tr} (\mathbf{X} \mathbf{Y} (\tau) \rho_{\beta}) \operatorname{Tr} (\mathbf{A}_j \rho_{\beta}) \pm \text{h.c.}|^2 + \left| \tau \operatorname{Tr} \left( \rho_{\beta} \mathbf{X} [\tilde{\mathbf{A}}_{j|\tau}, \mathbf{Y} (\tau)] \right) \pm \text{h.c.} \right|^2 \right) \quad (\text{C54})$$

$$\leq \frac{3n}{4} \left( 16\beta^2 \|\mathbf{X}\|^2 \|\mathbf{Y}\|^2 \Delta + 4\beta^2 \|\mathbf{X}\|^2 \|\mathbf{Y}\|^2 \Delta + 64\tau^2 \|\mathbf{X}\|^2 \|\mathbf{Y}\|^2 \Delta \right) \quad (\text{C55})$$

$$= 3n (5\beta^2 + 16\tau^2) \|\mathbf{X}\|^2 \|\mathbf{Y}\|^2 \Delta. \quad (\text{C56})$$

## Appendix D: Lower bound on SYK optimum

Consider a Hamiltonian weighted by i.i.d. Gaussian coefficients

$$\mathbf{H} = \frac{1}{\sqrt{m}} \sum_{i=1}^m g_i \mathbf{A}_i \quad \text{where } \mathbb{E} g_i^2 = 1 \quad (\text{D1})$$

where we here assume that all  $\mathbf{A}_i^2 = \|\mathbf{A}_i\|^2 \mathbf{I}$ . We show a lower bound for the maximal eigenvalue for such a Hamiltonian.

**Theorem D.1** (Lower bound on the maximal eigenvalue). *There is an absolute constant  $c_1$  such that*

$$\mathbb{E} \lambda_{\max}(\mathbf{H}) \geq \frac{\sqrt{m}}{4\sqrt{c_1} h_{\text{comm}}} (h_{\text{glo}}^2 - 16\Delta). \quad (\text{D2})$$

The commutation index  $\Delta = \Delta(\{\mathbf{A}_j\}_{j=1}^m)$  is defined in [Definition B.1](#). We have also defined the *commutation degree*

$$h_{\text{comm}} := \frac{1}{2} \sup_i \sum_{j=1}^m \frac{\|\mathbf{A}_j\| \cdot \|[\mathbf{A}_i, \mathbf{A}_j]\|}{\|\mathbf{A}_i\|} \quad (\text{D3})$$

and the *global norm*

$$h_{\text{glo}} := \sqrt{\frac{1}{m} \sum_{i=1}^m \|\mathbf{A}_i\|^2}. \quad (\text{D4})$$

Our results in the main text are only reported in the case all  $\|\mathbf{A}_i\| = 1$  for simplicity. Note that here, both  $h_{\text{comm}}$  and  $h_{\text{glo}}$  feature a quadratic sum (instead of a linear sum) due to the randomness of the Hamiltonian.

This immediately gives lower bounds for the SYK maximal eigenvalue.

**Corollary D.1.** *With high probability over the disorder, the maximum eigenvalue of the SYK model is*

$$\lambda_{\max}(\mathbf{H}_q^{\text{SYK}}) = \Omega(\sqrt{n/q}). \quad (\text{D5})$$

*Proof.* We have  $h_{\text{comm}} = \mathcal{O}(q \binom{n-1}{q-1})$  and  $m = \binom{n}{q}$  for SYK.  $\Delta = o(1)$  by Theorem B.2. The result then follows from Theorem D.1 and the concentration of the maximal eigenvalue (Fact C.2).  $\square$

Our theorem also lower bounds the maximal eigenvalue of a  $k$ -local quantum spin glass:

$$\mathbf{H}_k^{\text{SG}} = \frac{1}{\sqrt{3^k \binom{n}{k}}} \sum_{k\text{-local } \sigma} g_{\sigma} \sigma. \quad (\text{D6})$$

**Corollary D.2.** *With high probability over the disorder, the maximum eigenvalue of the  $k$ -local quantum spin glass model is*

$$\lambda_{\max}(\mathbf{H}_k^{\text{SG}}) = \Omega(\sqrt{n/k}) \quad (\text{D7})$$

when  $k \geq 3$ .

*Proof.* We have  $h_{\text{comm}} = \mathcal{O}(k 3^{k-1} \binom{n-1}{k-1})$  and  $m = 3^k \binom{n}{k}$  for the quantum spin glass described in Eq. (D6).  $\Delta$  is also less than  $1/16$  when  $k \geq 3$  by Proposition B.1. The result then follows from Theorem D.1 and the concentration of the maximal eigenvalue (Fact C.2).  $\square$

The strategy to prove Theorem D.1 is to lower bound the optimum by calculating the exponential

$$\begin{aligned} e^{\beta \mathbb{E} \lambda_{\max}(\mathbf{H})} &\approx \mathbb{E} e^{\beta \lambda_{\max}(\mathbf{H})} && (\text{Concentration of the maximal eigenvalue: Fact C.2}) \\ &\geq \mathbb{E} \frac{1}{N} \sum_i e^{\beta \lambda_i(\mathbf{H})} = \mathbb{E} \overline{\text{Tr}}[e^{\beta \mathbf{H}}], \end{aligned} \quad (\text{D8})$$

where we use  $\overline{\text{Tr}}[\cdot] := \frac{1}{N} \text{Tr}[\cdot]$  to denote the normalized trace. We begin by lower bounding the right-hand side.

**Lemma D.1** (Lower bounds on the exponential). *There is an absolute constant  $c_1$  such that for each  $\beta$ ,*

$$\mathbb{E} \overline{\text{Tr}}[e^{\beta \mathbf{H}}] \geq \exp \left( \frac{\beta^2 h_{\text{glo}}^2}{2} \left( 1 - \frac{c_1 \beta^2 h_{\text{comm}}}{2m} \right) \right). \quad (\text{D9})$$

In proving Lemma D.1 we will use the below two facts.

**Fact D.1** (Integration by parts). *For standard Gaussian random variable  $g$  and a function  $f : \mathbb{R} \rightarrow \mathbb{R}$  whose derivative is absolutely integrable w.r.t. the Gaussian measure, we have that*

$$\mathbb{E}[gf(g)] = \mathbb{E}[f'(g)]. \quad (\text{D10})$$

**Fact D.2** (Multivariate Hölder for random matrices e.g., [38, Fact A.1]). *For any family  $(\mathbf{X}_1, \dots, \mathbf{X}_k)$  of square random matrices, possibly statistically dependent, the product satisfies the trace inequality*

$$\mathbb{E} \overline{\text{Tr}} \left| \prod_{i=1}^k \mathbf{X}_i \right| = \left\| \prod_{i=1}^k \mathbf{X}_i \right\|_1 \leq \prod_{i=1}^k \|\mathbf{X}_i\|_{p_i} \quad \text{whenever} \quad \sum_{i=1}^k \frac{1}{p_i} = 1 \quad \text{and} \quad p_i \geq 0, \quad (\text{D11})$$

where

$$\|\mathbf{X}\|_p := (\mathbb{E} \overline{\text{Tr}}[|\mathbf{X}|^p])^{1/p}. \quad (\text{D12})$$

*Proof of Lemma D.1.* Take derivative w.r.t.  $\beta$ :

$$\frac{\partial}{\partial \beta} \mathbb{E} \overline{\text{Tr}}[e^{\beta \mathbf{H}}] = \mathbb{E} \overline{\text{Tr}}[\mathbf{H} e^{\beta \mathbf{H}}] \quad (\text{D13})$$

$$= \frac{1}{\sqrt{m}} \sum_{i=1}^m \mathbb{E} \overline{\text{Tr}}[g_i \mathbf{A}_i e^{\beta \mathbf{H}}] \quad (\text{D14})$$

$$= \frac{1}{\sqrt{m}} \sum_{i=1}^m \mathbb{E} \overline{\text{Tr}}[\mathbf{A}_i \partial_i e^{\beta \mathbf{H}}] \quad (\text{Integration by parts: Fact D.1})$$

$$= \frac{\beta}{m} \sum_{i=1}^m \int_0^1 \mathbb{E} \overline{\text{Tr}}[\mathbf{A}_i e^{\beta \mathbf{H}s} \mathbf{A}_i e^{\beta \mathbf{H}(1-s)}] ds \quad (\text{Derivative of matrix exponential [77]})$$

$$= \frac{\beta}{m} \sum_{i=1}^m \int_0^1 \mathbb{E} \overline{\text{Tr}}[\mathbf{A}_i^2 e^{\beta \mathbf{H}}] ds + \frac{\beta^2}{m} \sum_{i=1}^m \int_{s_1+s_2+s_3=1} \mathbb{E} \overline{\text{Tr}}[\mathbf{A}_i e^{\beta \mathbf{H}s_3} [\mathbf{H}, \mathbf{A}_i] e^{\beta \mathbf{H}s_2} e^{\beta \mathbf{H}s_1}] ds. \quad (\text{D15})$$

The last line “swaps” the  $\mathbf{A}_i$  through the exponential, resulting in errors written as commutator, as seen from

$$e^{\mathbf{X}}\mathbf{Y} - \mathbf{Y}e^{\mathbf{X}} = \int_0^1 e^{\mathbf{X}s}[\mathbf{X}, \mathbf{Y}]e^{\mathbf{X}(1-s)}ds \quad (\text{D16})$$

and setting  $\mathbf{X} = s\mathbf{H}$ . The intuition is that the commutator would be small due to the locality of  $\mathbf{A}_i$ . We further expand the second term

$$\int_{s_1+s_2+s_3=1} \mathbb{E} \overline{\text{Tr}}[\mathbf{A}_i e^{\beta \mathbf{H} s_3} [\mathbf{H}, \mathbf{A}_i] e^{\beta \mathbf{H} (s_1+s_2)}] ds \quad (\text{D17})$$

$$= \frac{1}{\sqrt{m}} \sum_{j=1}^m \int_{s_1+s_2+s_3=1} \mathbb{E} \overline{\text{Tr}}[\mathbf{A}_i e^{\beta \mathbf{H} s_3} [g_j \mathbf{A}_j, \mathbf{A}_i] e^{\beta \mathbf{H} (s_1+s_2)}] ds \quad (\text{D18})$$

$$= \frac{\beta}{m} \sum_{j=1}^m \int_{s_1+s_2+s_3+s_4=1} \mathbb{E} \overline{\text{Tr}}[\mathbf{A}_i e^{\beta \mathbf{H} s_4} \mathbf{A}_j e^{\beta \mathbf{H} s_3} [\mathbf{A}_j, \mathbf{A}_i] e^{\beta \mathbf{H} (s_1+s_2)}] ds + (\text{other insertions of } \mathbf{A}_j).$$

(Integration by parts: [Fact D.1](#))

Thus,

$$\left| \frac{\beta^2}{m} \sum_{i=1}^m \int_{s_1+s_2+s_3=1} \mathbb{E} \overline{\text{Tr}}[\mathbf{A}_i e^{\beta \mathbf{H} s_3} [\mathbf{H}, \mathbf{A}_i] e^{\beta \mathbf{H} s_2} e^{\beta \mathbf{H} s_1}] ds \right| \quad (\text{D19})$$

$$\leq (\text{const.}) \frac{\beta^3}{m^2} \sum_{i,j=1}^m \|\mathbf{A}_i\| \|\mathbf{A}_j\| \cdot \|[ \mathbf{A}_i, \mathbf{A}_j ]\| \cdot \mathbb{E} \overline{\text{Tr}}[e^{\beta \mathbf{H}}] \quad (\text{Fact D.2 and } e^{\mathbf{H}} \succ 0)$$

$$= (\text{const.}) \frac{\beta^3}{m^2} \sum_{i=1}^m \|\mathbf{A}_i\|^2 \sum_{j=1}^m \|\mathbf{A}_j\| \frac{\|[ \mathbf{A}_i, \mathbf{A}_j ]\|}{\|\mathbf{A}_i\|} \cdot \mathbb{E} \overline{\text{Tr}}[e^{\beta \mathbf{H}}] \quad (\text{D20})$$

$$\leq c_1 \frac{\beta^3}{m} h_{\text{glo}}^2 h_{\text{comm}} \cdot \mathbb{E} \overline{\text{Tr}}[e^{\beta \mathbf{H}}]. \quad (\text{By definitions of } h_{\text{glo}} \text{ and } h_{\text{comm}})$$

The second inequality uses a multivariate Hölder’s inequality for random matrices ([Fact D.2](#)), for moment parameters  $p = 1/s$  for each  $e^{\beta \mathbf{H} s}$ , and  $p = \infty$  for each  $\mathbf{A}_i, \mathbf{A}_j$  and  $[\mathbf{A}_i, \mathbf{A}_j]$ . Indeed,  $\|e^{\beta \mathbf{H} s}\|_{1/s} = (\mathbb{E} \overline{\text{Tr}}[|e^{\beta \mathbf{H}}|])^s = (\mathbb{E} \overline{\text{Tr}}[e^{\beta \mathbf{H}}])^s$ . Also, the “(const.)” notation absorbs the absolute numerical constants arising from the insertions of  $\mathbf{A}_j$  and the integration over  $\sum_\ell s_\ell = 1$ . Thus, defining  $f(\beta) := \mathbb{E} \overline{\text{Tr}}[e^{\beta \mathbf{H}}]$ ,

$$\frac{\partial}{\partial \beta} f(\beta) \geq \beta h_{\text{glo}}^2 f(\beta) - c_1 \frac{\beta^3}{m} h_{\text{glo}}^2 h_{\text{comm}} f(\beta) \quad (\text{D21})$$

$$\implies f(\beta) \geq \exp \left( \int_0^\beta \beta' h_{\text{glo}}^2 \left( 1 - c_1 \frac{\beta'^2}{m} h_{\text{comm}} \right) d\beta' \right) f(0) \quad (\text{Gronwall's differential inequality})$$

$$\geq \exp \left( \frac{\beta^2 h_{\text{glo}}^2}{2} \left( 1 - \frac{c_1 \beta^2 h_{\text{comm}}}{2m} \right) \right), \quad (\text{By the initial condition } f(0) = 1)$$

which concludes the proof.  $\square$

One immediate corollary of this lemma is the following, which follows from evaluating Eq. (D9) at  $\beta = \sqrt{\frac{m}{c_1 h_{\text{comm}}}}$ .

**Corollary D.3** (A good  $\beta$ ).

$$\mathbb{E} \overline{\text{Tr}}[e^{\beta_{\max} \mathbf{H}}] \geq \exp \left( \frac{h_{\text{glo}}^2 m}{4c_1 h_{\text{comm}}} \right) \quad \text{at} \quad \beta_{\max} := \sqrt{\frac{m}{c_1 h_{\text{comm}}}}. \quad (\text{D22})$$

With the preliminaries in place we are now able to prove [Theorem D.1](#). We will also use eigenvalue concentration bounds proved in [Appendix C](#).

*Proof of Theorem D.1.* We have the lower bound:

$$\begin{aligned} \ln \mathbb{E} e^{\beta_{\max} \lambda_{\max}(\mathbf{H})} &\geq \ln \mathbb{E} \overline{\text{Tr}}[e^{\beta_{\max} \mathbf{H}}] \\ &\geq \beta_{\max} \cdot \frac{h_{\text{glo}}^2 \sqrt{m}}{4\sqrt{c_1 h_{\text{comm}}}}. \end{aligned} \quad (\text{D23}) \quad (\text{Corollary D.3})$$

Also, we move the expectation to the exponent by concentration of the maximal eigenvalue

$$\begin{aligned}\mathbb{E}e^{\beta\lambda_{\max}(\mathbf{H})} &= e^{\beta\mathbb{E}\lambda_{\max}(\mathbf{H})}\mathbb{E}e^{\beta\lambda_{\max}(\mathbf{H})-\beta\mathbb{E}\lambda_{\max}(\mathbf{H})} \\ &\leq e^{\beta\mathbb{E}\lambda_{\max}(\mathbf{H})}e^{4\beta^2\Delta}.\end{aligned}\tag{D24}$$

(Fact C.2 and Lemma C.1)

Rearrange and take the logarithm to conclude the proof.  $\square$

## Appendix E: Circuit lower bound for SYK model

In this section we show that low-energy states of random strongly interacting fermionic Hamiltonians have high circuit complexity. In particular, we show a circuit lower bound on the low energy states of the the SYK model. It was previously described in Eq. (1), but we repeat its definition here for convenience.

**Definition E.1.** Let  $\mathcal{S}_q^n$  denote the set of degree- $q$  Majorana operators on  $n$  fermionic modes. The SYK $_q$  model is a random ensemble of Hamiltonians defined by

$$\mathbf{H}_q^{\text{SYK}} = \frac{1}{\sqrt{\binom{n}{q}}} \sum_{A \in \mathcal{S}_q^n} g_A \mathbf{A} \quad , \quad g_A \sim_{i.i.d.} \mathcal{N}(0, 1).\tag{E1}$$

**Theorem E.1.** (SYK model low-energy states have high circuit complexity.) Let  $\text{circ}(G)$  denote the set of unitaries generated by quantum circuits with at most  $G$  gates each taken from a finite universal set of 2-local unitary gates. Fix an arbitrary initial state  $|\phi\rangle$ . With high probability, for any even  $q \geq 2$ , it holds that the minimum circuit complexity to construct a state achieving at least  $t\sqrt{n}$  on  $\mathbf{H}_q^{\text{SYK}}$  is at least

$$\min \{G : \exists U \in \text{circ}(G), \langle \phi | U^\dagger \mathbf{H}_q^{\text{SYK}} U | \phi \rangle \geq t\sqrt{n}\} = \tilde{\Omega}_q(n^{(q/2)+1}t^2).\tag{E2}$$

Meanwhile, we can recall Corollary D.1 from Appendix D, which gives a lower bound  $\lambda_{\max}(\mathbf{H}_q^{\text{SYK}}) = \Omega_q(\sqrt{n})$  on the maximum eigenvalue of SYK.

The proof of Theorem E.1 will proceed via a concentration argument, followed by a union bound over the circuit family. This resembles the circuit lower bound of [79, Appendix D]. We establish the necessary concentration now, which relies crucially on Theorem B.1 concerning the commutation index of low-degree Majorana operators.

**Lemma E.1.** Fix any state  $|\psi\rangle$ . The energy  $\langle \psi | \mathbf{H}_q^{\text{SYK}} | \psi \rangle$  sharply concentrates:

$$\mathbb{P}(\langle \psi | \mathbf{H}_q^{\text{SYK}} | \psi \rangle \geq t) \leq \exp(-\Omega_q(n^{q/2}t^2)).\tag{E3}$$

*Proof.* Since a sum of Gaussians is Gaussian, we have

$$\mathbb{P}(\langle \psi | \mathbf{H}_q^{\text{SYK}} | \psi \rangle \geq t) \leq \exp(-t^2/2\sigma^2)\tag{E4}$$

where

$$\sigma^2 = \frac{1}{\binom{n}{q}} \sum_{A \in \mathcal{S}_q^n} \langle \psi | \mathbf{A} | \psi \rangle^2 \leq \Delta(\mathcal{S}_q^n) \leq \frac{(q)!}{2^{q/2}(q/2)!} n^{-q/2} + \mathcal{O}(e^{\mathcal{O}(q \log q)} n^{-(q/2)-1}) = \mathcal{O}_q(n^{-q/2}).\tag{E5}$$

Here  $\Delta$  is from Definition B.1, and we used the upper bound of Theorem B.1.  $\square$

*Proof of Theorem E.1.* Let  $M$  be the number of gates in the universal gate set. Then the number of circuits in  $\text{circ}(G)$  is at most

$$|\text{circ}(G)| \leq \left( M \binom{n}{2} \right)^G = \exp(\mathcal{O}(G \log(n))).\tag{E6}$$

Performing a union bound on Lemma E.1 yields

$$\mathbb{P} \left[ \max_{U \in \text{circ}(G)} \langle \phi | U^\dagger \mathbf{H}_q^{\text{SYK}} U | \phi \rangle \geq t\sqrt{n} \right] \leq \exp \left( -\Omega_q(n^{q/2+1}t^2) + \mathcal{O}(G \log(n)) \right).\tag{E7}$$

For this to be non-vanishing in  $n$ , it requires

$$G = \Omega_q(n^{q/2+1}t^2/\log n).\tag{E8}$$

$\square$

**Remark E.1.1.** The proof above can be extended to gates with continuous parameters by forming an  $\epsilon$ -net over the gates. This comes at the cost of additional  $\log(1/\epsilon)$  factors in the bound of Theorem E.1.

**Remark E.1.2.** A similar union bound can be applied to show that any state from the set of Gaussian states cannot be a near ground state for the SYK Hamiltonian for  $q \geq 4$ , since an  $\epsilon$ -net over the set of Gaussian states has cardinality  $\exp(\tilde{\mathcal{O}}(n^2 + \text{poly log}(1/\epsilon)))$ . This reproduces results from prior works [29, 80].

### 1. Relation to NLTS results

Our circuit lower bound is closely related to the study of ‘no low-energy trivial states’ (NLTS) Hamiltonians. Introduced in [51], a Hamiltonian  $H = \sum_i g_i A_i$  has the NLTS property if there is no constant-depth circuit preparing a state whose energy is above the ground energy by less than some constant fraction of the  $\ell_1$  norm  $\sum_i |g_i|$ . Such Hamiltonians were first proven to exist in [36] using quantum LDPC codes.

The circuit lower bounds we give are not quite comparable to the traditional notion of NLTS. This is because we compare the energy of our low-energy states to the Hamiltonian’s maximum eigenvalue rather than the  $\ell_1$  norm of the coefficients. Unlike the quantum code Hamiltonian studied in [36, 37], the SYK model is highly frustrated and thus the operator norm and  $\ell_1$  norms have vastly different scalings:  $\Theta(n^2)$  and  $\Theta(\sqrt{n})$ , respectively.

Despite these differences from the standard NLTS setting, the circuit lower bounds we can establish are much stronger in two ways when compared to current progress on NLTS [36, 37, 81–83]. First, our circuit lower bounds hold for states at *any* energy which is a constant fraction of the ground state energy, rather than for states below some constant-fraction energy threshold. Second, we can achieve arbitrary polynomial circuit depth lower bounds, whereas current constructions of NLTS only give a logarithmic depth lower bound.

### 2. Other notions of non-triviality

Though our focus so far has been on quantum circuit lower bounds, our results readily generalize to lower bounds for other classes of ansatzes via the construction of covering nets. We begin by discussing tensor networks, focusing on matrix product states (MPSs) as a particular example. Implemented at any finite precision the number of configurations of a matrix product state on  $n$  sites grows with the *bond dimension*  $\chi$  as  $|\{|\psi_j\rangle\}| = \exp(\Theta(\chi^2 + \log n))$ . It is thus apparent from the same argument that the minimum bond dimension such that there is an MPS achieving an energy  $t\sqrt{n}$  is

$$\chi = \Omega_q \left( n^{q/4+1/2} t \right) \quad (\text{E9})$$

with high probability. Similarly, a classical neural network representation of the state with  $W$  weights has a number of configurations growing as  $|\{|\psi_j\rangle\}| = \exp(\Theta(W))$ , yielding the growth condition to achieve an energy  $t\sqrt{n}$  with high probability:

$$W = \Omega_q \left( n^{q/2+1} t^2 \right). \quad (\text{E10})$$

### 3. Product state approximations for spin Hamiltonians

It is worth pointing out that there cannot be a  $k$ -local spin Hamiltonian with the property in Theorem E.1. For any traceless  $k$ -local spin Hamiltonian  $H$ , there is a product state achieving energy at least  $\lambda_{\max}(H)/3^k$ . The argument is imported from [24, proof of Theorem 2], and the proof technique bears a remarkable resemblance to the classical shadows protocol [84], which provides a learning algorithm for  $k$ -local spin operators.

**Proposition E.1.** *For any  $k$ -local Hamiltonian  $H$  on  $n$  qubits, there is a product state  $|\psi\rangle$  achieving energy*

$$\langle \psi | H | \psi \rangle \geq \lambda_{\max}(H)/3^k. \quad (\text{E11})$$

*Proof.* Let  $|\phi\rangle$  be the true (possibly entangled) maximum-energy state achieving

$$\langle \phi | H | \phi \rangle = \lambda_{\max} \quad (\text{E12})$$

For each qubit, pick a random basis out of  $\{\sigma_X, \sigma_Y, \sigma_Z\}$ , and measure in this basis. This gives a product state of single-qubit stabilizer states. Let  $\rho$  be the resulting ensemble of pure product states we will analyze  $\text{Tr}(H\rho)$ . It turns out that

$$\rho = \mathcal{E}_{1/3}^{\otimes n}(|\phi\rangle\langle\phi|) \quad (\text{E13})$$

where  $\mathcal{E}_p$  is the depolarizing channel

$$\mathcal{E}_p(\tau) = p\tau + (1-p)\mathbb{1} \quad (\text{E14})$$

Thus

$$\text{Tr}(H\rho) = \text{Tr}\left(H\mathcal{E}_{1/3}^{\otimes n}(|\phi\rangle\langle\phi|)\right) = \text{Tr}\left(\mathcal{E}_{1/3}^{\otimes n}(H)|\phi\rangle\langle\phi|\right) = \langle\phi|(H/3^k)|\phi\rangle = \lambda_{\max}(H)/3^k \quad (\text{E15})$$

Here we used that  $H$  is traceless and  $k$ -local.  $\square$