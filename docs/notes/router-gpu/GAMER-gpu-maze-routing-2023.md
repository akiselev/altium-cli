

# Discernibility of topological variations for networked LTI systems based on observed output trajectories

Yuqing Hao <sup>a</sup>, Qingyun Wang <sup>a</sup>, Zhisheng Duan <sup>b</sup>, Guanrong Chen <sup>c</sup>

<sup>a</sup>*Department of Dynamics and Control, Beihang University, Beijing, 100191, China*

<sup>b</sup>*State Key Laboratory for Turbulence and Complex Systems, Department of Mechanics and Engineering Science, College of Engineering, Peking University, Beijing 100871, China*

<sup>c</sup>*Department of Electronic Engineering, City University of Hong Kong, Hong Kong, China*

## Abstract

In this paper, the possibility of detecting topological variations by observing output trajectories from networked linear time-invariant systems is investigated, where the network topology can be general, but the nodes have identical higher-dimensional dynamics. A necessary and sufficient condition on the discernibility of topological variations is derived, in terms of the eigenspaces of the original and the modified network configurations. By taking the specific network structures into consideration, some lower-dimensional conditions are derived, which reveal how the network topologies, sensor locations, node-system dynamics and output, as well as inner interactions altogether affect the discernibility. Furthermore, the output discernibility of topological changes for networked multi-agent systems is revisited, showing that some criterion reported in the literature does not hold. Consequently, a modified necessary and sufficient condition is established. The effectiveness of the results is demonstrated through several examples.

*Key words:* Networked systems, topological variation, discernibility, output trajectory.

## 1 Introduction

In the last two decades, the study of networked systems has gained enormous popularity from the communities in engineering, information technology, mathematics, sociology, biology, and physics. Examples of networked systems in practical applications include wireless communication networks [28], networked robotics [10], global transportation networks [2], power generation and distribution networks [27], and so on.

It is well known that topological structure has significant effects on the performance of the underlying networked

systems. In practical networks, the failures of network components or denial-of-service attacks may result in topological variations [1, 30], which can affect the network performance [12, 14, 25, 26], sometimes even have disastrous impacts on the secure and reliable operation. One example is the catastrophic power outage in southern Italy in 2003, which was reportedly caused by failures of some high-voltage transmission lines [5]. Therefore, there is a need of detecting topological variations in time to protect the networked systems.

Detecting topological variations of networked systems has received compelling attention, with many efficient approaches proposed [7–9, 16, 21, 23]. A method to detect and isolate link failures was developed in [21], which is based on the observed jumps in the derivatives of the output responses of a subset of nodes in networked linear time-invariant (LTI) systems. In [9] and [23], link failures in a network synchronization process were detected from noisy local measurements by using a maximum a posteriori probability detection technique. A distributed fault detection and isolation filter was designed in [8] for a network of heterogeneous multi-agent systems. A structural anomaly detection algorithm was proposed in [7], which

\* This work is supported by the National Natural Science Foundation of China under Grants 12172020, 11932003, T2121002, in part by the Beijing Natural Science Foundation under Grant 1222010, and in part by the Hong Kong Research Grants Council under the GRF Grant CityU 11206320. The material in this paper was not presented at any conference.

*Email addresses:* haoyq@buaa.edu.cn (Yuqing Hao), nmqingyun@163.com (Qingyun Wang), duanzs@pku.edu.cn (Zhisheng Duan), eegchen@cityu.edu.hk (Guanrong Chen).

is capable of detecting topological changes in dynamical networks. In [16], a diffusion protocol for networked multi-agent systems was constructed and applied to link failure detection. Note that most efforts are devoted to developing detection algorithms. However, the possibility of detecting a topological change by observing the network behavior is even more important but challenging [18].

Recently, the discernibility of topological variations has become a focal topic for investigation. The detectability of single link failures in a multi-agent network was investigated in [19], which was generalized to multi-link failures in [20]. In [3], some conditions for detecting node or link disconnections of integrator networks were established. The effects of an edge or a node disconnection on a multi-agent consensus network were investigated in [24]. The results were further extended to the detection and identification of edge disconnections in [17]. Note that most if not all existing results on the discernibility of topological variations are derived under the assumption that all nodes are one-dimensional [3, 17, 24]. However, in real-world networks, nodes typically have higher-dimensional states, which are coupled via multi-dimensional communication channels [6]. In such situations, the discernibility of topological changes becomes much more complicated and challenging.

To date, there has been little work on the discernibility of topological changes for networked higher-dimensional systems. The discernibility of topological changes for networks of linear dynamical systems was studied in [4]. A necessary and sufficient condition on the discernibility of topological changes for a network of differential-algebraic systems was established in [18], with the indiscernible initial states characterized. Very recently, some lower-dimensional conditions on the discernibility of topological variations for networked LTI systems were established in [13]. Note that all the above-mentioned works focus on detecting topological changes from the observation of the whole network state. However, complete observation of the full state is always unrealistic. In many practical situations, only partial information about the node-system state is accessible, and only a subset of nodes are available for measurement. Hence, detecting topological variations by observing output trajectories of networks has broader applicability in practice. The output discernibility of topological variations for networks of linear dynamical systems was investigated in [4], which requires the network topology to be undirected. In [29], the detectability and isolability of topology failures for a network based on the observed output behavior were studied, where the edge weights can be unknown.

In this paper, the possibility of detecting topological variations by observing output trajectories of networked LTI systems is investigated. The contribution of this paper is four-fold. First, the network topology can be

general, directed and weighted. The node-systems have identical higher-dimensional linear dynamics. Differing from [4, 18] and [24], this paper allows directed network topologies. Second, a necessary and sufficient condition on the discernibility of topological variations is derived, in terms of the eigenspaces of the original and the modified network configurations. Third, some lower-dimensional conditions on the discernibility of topological variations are established, which explicitly illustrate how the network topologies, sensor locations, node-system dynamics and output, as well as inner interactions altogether affect the discernibility of the networks. Compared with [13] and [18], which assume that all state variables of the networks are accessible, these new conditions verify the possibility of detecting topological changes using the output of a subset of nodes, thus are more general and have broader applicability in practice. Fourth, the output discernibility of topological variations for multi-agent systems is revisited, revealing that the sufficiency of the criterion given in [4] does not hold, and consequently a modified necessary and sufficient condition is derived.

The remainder of this paper is organized as follows: Some preliminaries and the model description are given in Section 2. Some eigenspace-based conditions on the discernibility of topological changes are developed in Section 3. Some lower-dimensional conditions on the discernibility of topological variations are established in Section 4. The output discernibility of topological changes for multi-agent systems is revisited in Section 5, with a modified condition derived. Finally, conclusions are drawn in Section 6.

## 2 Preliminaries and model description

Some preliminaries and the model description are introduced in this section.

### 2.1 Notation and definitions

Let  $\mathbb{N}$ ,  $\mathbb{R}$  and  $\mathbb{C}$  be the fields of integers, real and complex numbers, respectively. Let  $I_n$  be the identity matrix of size  $n \times n$ ,  $e_i$  be the vector with all zero entries except for  $[e_i]_i = 1$ ,  $\mathbf{0}_n$  be the  $n \times 1$  vector with all zero entries, and  $\text{diag}\{a_1, a_2, \dots, a_n\}$  be a diagonal matrix with diagonal entries  $a_1, a_2, \dots, a_n$ . The linear span of a set of vectors over the complex field is denoted as  $\text{span}\{\cdot\}$ . Moreover, for a matrix  $A \in \mathbb{R}^{n \times n}$ ,  $\sigma(A) = \{\lambda_1, \dots, \lambda_r\}$  denotes the set of all its eigenvalues,  $S(\lambda_i|A) = \text{span}\{x \in \mathbb{C}^n | Ax = \lambda_i x\}$  denotes the eigenspace corresponding to  $\lambda_i$ , and  $\tau(\lambda_i|A)$  denotes the geometric multiplicity of  $\lambda_i$ . The null space of a real matrix  $M \in \mathbb{R}^{n \times m}$  is denoted as  $\mathcal{N}(M)$ . The dimension of a vector space is denoted by  $\mathbf{dim}$ . Let  $A \otimes B$  be the Kronecker product of matrices  $A$  and  $B$ . Given a set of matrices  $\{A_1, \dots, A_n\}$ , if they have the same column

dimension, then  $\text{col}(A_1, \dots, A_n) = [A_1^T, \dots, A_n^T]^T$ . Let  $V_1 + V_2$  and  $V_1 \oplus V_2$  be the sum and the direct sum of spaces  $V_1$  and  $V_2$ , respectively. Matrices, if their dimensions are not explicitly indicated, are assumed to be compatible for algebraic operations.

A directed and weighted graph  $\mathcal{G} = (\mathcal{V}, \mathcal{E}, \mathcal{W})$  consists of a node set  $\mathcal{V} = \{1, \dots, n\}$ , an edge set  $\mathcal{E} \subset \mathcal{V} \times \mathcal{V}$ , and a weight matrix  $\mathcal{W} = [w_{ij}] \in \mathbb{R}^{n \times n}$ . Note that  $(j, i) \in \mathcal{E}$  if and only if  $w_{ij} \neq 0$ . The adjacency matrix of graph  $\mathcal{G}$  is denoted by  $\mathcal{A}(\mathcal{G}) = [a_{ij}] \in \mathbb{R}^{n \times n}$ , where  $a_{ij} = w_{ij}$  if  $(j, i) \in \mathcal{E}$ , and  $a_{ij} = 0$  otherwise.

**Definition 1** [22] A vector  $x_m$  is called an  $m$ th-order generalized eigenvector of matrix  $A$  corresponding to the eigenvalue  $\lambda$  if  $(A - \lambda I)^m x_m = 0$  and  $(A - \lambda I)^{m-1} x_m \neq 0$ . Also,  $x_1, \dots, x_g$  form a Jordan chain of  $A$  with top vector  $x_1$ , where the maximum number  $g$  is called the length of this Jordan chain.

**Definition 2** [13] Let  $A \in \mathbb{C}^{n \times n}$ ,  $H \in \mathbb{C}^{n \times n}$ , and  $\lambda$  be an eigenvalue of  $A$ . If vectors  $x_1, x_2, \dots, x_\theta$  satisfy  $(\lambda I - A)x_1 = 0$  and  $(\lambda I - A)x_{i+1} = Hx_i$  for  $i \in \{1, \dots, \theta-1\}$ , then  $x_1, x_2, \dots, x_\theta$  constitute a generalized Jordan chain of  $A$  about  $H$  corresponding to the eigenvalue  $\lambda$ , where  $x_1$  is the top vector, and the maximum number  $\theta$  is the length of this generalized Jordan chain.

### 2.2 Model description

Consider a network consisting of  $N$  identical nodes, with a general directed and weighted topology  $\mathcal{G} = (\mathcal{V}, \mathcal{E}, \mathcal{W})$ . Each node is represented by an LTI system:

$$\begin{cases} \dot{x}_i = Ax_i + \sum_{j=1}^N w_{ij} H x_j, \\ y_i = Cx_i, \end{cases} \quad i = 1, 2, \dots, N, \quad (1)$$

where  $x_i \in \mathbb{R}^n$  is the state vector,  $A \in \mathbb{R}^{n \times n}$  is the state matrix describing the dynamics of the node-systems;  $w_{ij} \in \mathbb{R}$  represents the coupling strength between nodes  $i$  and  $j$ ,  $H \in \mathbb{R}^{n \times n}$  denotes the inner coupling matrix describing the interconnections among components of  $x_j$ ;  $y_i \in \mathbb{R}^p$  is the output vector, and  $C \in \mathbb{R}^{p \times n}$  is the output matrix. To avoid trivial situations, always assume  $N \geq 2$  in this paper. Assume that  $w_{ii} = 0$ , and  $w_{ij} \neq 0$  if there is an edge from node  $j$  to node  $i$ , otherwise  $w_{ij} = 0$ , for all  $i, j = 1, 2, \dots, N$ . Let  $L = [w_{ij}] \in \mathbb{R}^{N \times N}$ , which represents the network topology for the networked systems (1). Let  $\Delta = \text{diag}\{\delta_1, \delta_2, \dots, \delta_N\}$  be an index matrix indicating the sensor locations, where  $\delta_i = 1$  if node  $i$  is available for measurement, but otherwise  $\delta_i = 0$ , for  $i = 1, 2, \dots, N$ . Moreover, let  $X = [x_1^T, x_2^T, \dots, x_N^T]^T$  and  $Y = [y_1^T, y_2^T, \dots, y_N^T]^T$  be the whole state and all output of the networked systems, respectively. Then, the networked systems (1) can be rewritten in a compact

form as

$$\begin{cases} \dot{X} = \Phi X, \\ Y = \Psi X, \end{cases} \quad (2)$$

where

$$\Phi = I_N \otimes A + L \otimes H, \quad \Psi = \Delta \otimes C. \quad (3)$$

## 3 Eigenspace-based conditions on the $\Psi$ -discernibility of topological changes

The effect of topological changes on the network output is investigated. In particular, the interest is in characterizing the topological changes that do not alter the output trajectories (for certain initial states). A topological change caused by a removal/addition of an edge, or a change in an edge weight, results in a new network

$$\begin{cases} \dot{\bar{X}} = \bar{\Phi} \bar{X}, \\ \bar{Y} = \bar{\Psi} \bar{X}, \end{cases} \quad (4)$$

with

$$\bar{\Phi} = I_N \otimes A + \bar{L} \otimes H. \quad (5)$$

Now, the concept of  $\Psi$ -indiscernible pair of initial states is introduced.

**Definition 3** [4] Consider two networked systems (2)-(3) and (4)-(5). A pair of initial states  $(X_0, \bar{X}_0) \in \mathbb{R}^{Nn} \times \mathbb{R}^{Nn}$  is called  $\Psi$ -indiscernible with respect to the topological change  $L \rightarrow \bar{L}$  if and only if  $\Psi e^{\Phi t} X_0 = \bar{\Psi} e^{\bar{\Phi} t} \bar{X}_0$ , for all  $t \geq 0$ .

Note that  $(X_0, \bar{X}_0) = (\mathbf{0}_{Nn}, \mathbf{0}_{Nn})$  is always a  $\Psi$ -indiscernible pair (irrespective of the specific topological variation), which is called the trivial indiscernible pair. According to whether a nontrivial  $\Psi$ -indiscernible pair exists, topological changes are classified into two groups as follows.

**Definition 4** [4] For the networked system (2)-(3), a topological change  $L \rightarrow \bar{L}$  is called  $\Psi$ -discernible if there is no (nontrivial)  $\Psi$ -indiscernible pair of initial states. Otherwise, it is called  $\Psi$ -indiscernible.

Let  $\mu$  be an eigenvalue of  $\Phi$  with the corresponding eigenspace  $S(\mu|\Phi)$ , and let  $\bar{\mu}$  be an eigenvalue of  $\bar{\Phi}$  with the associated eigenspace  $S(\bar{\mu}|\bar{\Phi})$ . In what follows, a necessary and sufficient condition on the  $\Psi$ -discernibility of topological changes is established in terms of the eigenspaces of the original and the modified networks.

**Theorem 1** Consider the networked system (2)-(3). A topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -discernible if and only if for all  $\mu \in \sigma(\Phi) \cup \sigma(\bar{\Phi})$ , the following two conditions hold simultaneously:

- (1)  $S(\mu|\Phi) \cap S(\mu|\bar{\Phi}) = \{\mathbf{0}_{Nn}\}$ ;
- (2)  $\mathcal{N}(\Psi) \cap \{S(\mu|\Phi) \oplus S(\mu|\bar{\Phi})\} = \{\mathbf{0}_{Nn}\}$ .

**Proof:** It is easy to verify that the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -discernible if and only if  $(diag\{\Phi, \bar{\Phi}\}, [\Psi - \Psi])$  is observable.

Necessity: If there exists  $\mu^* \in \sigma(\Phi) \cup \sigma(\bar{\Phi})$  such that  $S(\mu^*|\Phi) \cap S(\mu^*|\bar{\Phi}) \neq \{\mathbf{0}_{Nn}\}$ , then there exists a nonzero vector  $x \in S(\mu^*|\Phi) \cap S(\mu^*|\bar{\Phi})$ . It is easy to verify that  $\Phi x = \mu^* x$  and  $\bar{\Phi} x = \mu^* x$ . Thus, there exists an eigenpair  $(\mu^*, [x^T \ x^T]^T)$  satisfying

$$\begin{bmatrix} \Phi \\ \bar{\Phi} \end{bmatrix} \begin{bmatrix} x \\ x \end{bmatrix} = \mu^* \begin{bmatrix} x \\ x \end{bmatrix} \text{ and } \begin{bmatrix} \Psi - \Psi \end{bmatrix} \begin{bmatrix} x \\ x \end{bmatrix} = \mathbf{0},$$

which implies that  $(diag\{\Phi, \bar{\Phi}\}, [\Psi - \Psi])$  is unobservable. Therefore, the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible.

Assume that for all  $\mu \in \sigma(\Phi) \cup \sigma(\bar{\Phi})$ ,  $S(\mu|\Phi) \cap S(\mu|\bar{\Phi}) = \{\mathbf{0}_{Nn}\}$ . If there exists  $\mu^* \in \sigma(\Phi) \cup \sigma(\bar{\Phi})$  such that  $\mathcal{N}(\Psi) \cap \{S(\mu^*|\Phi) \oplus S(\mu^*|\bar{\Phi})\} \neq \{\mathbf{0}_{Nn}\}$ , then there exists a nonzero vector  $x \in \{S(\mu^*|\Phi) \oplus S(\mu^*|\bar{\Phi})\}$  such that  $\Psi x = \mathbf{0}$ . Let  $x = x_1 + x_2$ , where  $x_1 \in S(\mu^*|\Phi)$  and  $x_2 \in S(\mu^*|\bar{\Phi})$ . From  $x \neq \mathbf{0}_{Nn}$ , it follows that  $x_1$  and  $x_2$  are not both  $\mathbf{0}_{Nn}$ , as discussed in the following three cases:

- If  $x_2 = \mathbf{0}_{Nn}$  and  $x_1 \neq \mathbf{0}_{Nn}$ , it is easy to verify that  $\Phi x_1 = \mu^* x_1$  and  $\Psi x_1 = \mathbf{0}$ . Then, there exists a nonzero vector  $[x_1^T \ \mathbf{0}_{Nn}^T]^T$  satisfying

$$\begin{bmatrix} \Phi \\ \bar{\Phi} \end{bmatrix} \begin{bmatrix} x_1 \\ \mathbf{0}_{Nn} \end{bmatrix} = \mu^* \begin{bmatrix} x_1 \\ \mathbf{0}_{Nn} \end{bmatrix} \text{ and } \begin{bmatrix} \Psi - \Psi \end{bmatrix} \begin{bmatrix} x_1 \\ \mathbf{0}_{Nn} \end{bmatrix} =$$

$\mathbf{0}$ , which implies that  $(diag\{\Phi, \bar{\Phi}\}, [\Psi - \Psi])$  is unobservable. Thus, the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible.

- If  $x_1 = \mathbf{0}_{Nn}$  and  $x_2 \neq \mathbf{0}_{Nn}$ , one can similarly prove that the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible.
- If  $x_1 \neq \mathbf{0}_{Nn}$  and  $x_2 \neq \mathbf{0}_{Nn}$ , then  $\Phi x_1 = \mu^* x_1$ ,  $\bar{\Phi} x_2 = \mu^* x_2$  and  $\Psi(x_1 + x_2) = \mathbf{0}$ . Consequently, there exists a nonzero vector  $[x_1^T \ -x_2^T]^T$  satisfying

$$\begin{bmatrix} \Phi \\ \bar{\Phi} \end{bmatrix} \begin{bmatrix} x_1 \\ -x_2 \end{bmatrix} = \mu^* \begin{bmatrix} x_1 \\ -x_2 \end{bmatrix} \text{ and } \begin{bmatrix} \Psi - \Psi \end{bmatrix} \begin{bmatrix} x_1 \\ -x_2 \end{bmatrix} =$$

$\mathbf{0}$ , which implies that  $(diag\{\Phi, \bar{\Phi}\}, [\Psi - \Psi])$  is unobservable. Thus, the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible.

Therefore, if there exists  $\mu^* \in \sigma(\Phi) \cup \sigma(\bar{\Phi})$  such that  $\mathcal{N}(\Psi) \cap \{S(\mu^*|\Phi) \oplus S(\mu^*|\bar{\Phi})\} \neq \{\mathbf{0}_{Nn}\}$ , then the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible.

Sufficiency: If the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible, then there exists an eigenpair of  $diag\{\Phi, \bar{\Phi}\}$ , denoted as  $(\mu^*, \eta)$ , such that  $[\Psi - \Psi]\eta =$

$\mathbf{0}$ . Let  $\mathbf{0} \neq \eta = [\eta_1^T \ \eta_2^T]^T$ , where  $\eta_1, \eta_2 \in \mathbb{C}^{Nn}$ . Then,  $\eta_1$  and  $\eta_2$  are not both  $\mathbf{0}_{Nn}$ , and they satisfy

$$\begin{cases} \Phi \eta_1 = \mu^* \eta_1, \\ \bar{\Phi} \eta_2 = \mu^* \eta_2, \\ \Psi(\eta_1 - \eta_2) = \mathbf{0}. \end{cases}$$

It follows that  $\eta_1 \in S(\mu^*|\Phi)$  and  $-\eta_2 \in S(\mu^*|\bar{\Phi})$ , thus  $\eta_1 - \eta_2 \in \{S(\mu^*|\Phi) + S(\mu^*|\bar{\Phi})\}$ . If  $S(\mu^*|\Phi) \cap S(\mu^*|\bar{\Phi}) \neq \{\mathbf{0}_{Nn}\}$ , then condition (1) does not hold. If  $S(\mu^*|\Phi) \cap S(\mu^*|\bar{\Phi}) = \{\mathbf{0}_{Nn}\}$ , then  $\eta_1 - \eta_2 \in \{S(\mu^*|\Phi) \oplus S(\mu^*|\bar{\Phi})\}$ . Noting that  $\eta_1$  and  $\eta_2$  are not both  $\mathbf{0}_{Nn}$ , one can conclude that  $\eta_1 - \eta_2 \neq \mathbf{0}_{Nn}$ . Thus,  $\mathbf{0}_{Nn} \neq \eta_1 - \eta_2 \in \mathcal{N}(\Psi) \cap \{S(\mu^*|\Phi) \oplus S(\mu^*|\bar{\Phi})\}$ , which implies that condition (2) does not hold. Therefore, if the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible, then at least one condition in Theorem 1 does not hold. ■

**Remark 1** The possibility of detecting topological variations by observing network states was investigated in [18], where it was claimed that the topological variation is always-discernible if and only if  $\Phi$  and  $\bar{\Phi}$  have no common eigenpairs. This condition can be reproduced from Theorem 1 as a special case with  $\Psi = I$ . Theorem 1 extends the results in [18] to a more general and practical case of detecting topological changes from the observation of output trajectories. This nontrivial extension is of great significance for engineering applications.

**Remark 2** Noting that  $S(\mu|\Phi) \subseteq \{S(\mu|\Phi) \oplus S(\mu|\bar{\Phi})\}$ , the second condition in Theorem 1 requires that  $\mathcal{N}(\Psi) \cap S(\mu|\Phi) = \{\mathbf{0}_{Nn}\}$  for all  $\mu \in \sigma(\Phi)$ , which implies that  $(\Phi, \Psi)$  is observable. Similarly, since  $S(\mu|\bar{\Phi}) \subseteq \{S(\mu|\Phi) \oplus S(\mu|\bar{\Phi})\}$ , Theorem 1 requires the observability of  $(\bar{\Phi}, \Psi)$  as well. Therefore, the observability of  $(\Phi, \Psi)$  and  $(\bar{\Phi}, \Psi)$  is necessary for the  $\Psi$ -discernibility of the topological variation. Moreover, if  $\Phi$  and  $\bar{\Phi}$  have no common eigenvalues, then the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -discernible if and only if both  $(\Psi, \Phi)$  and  $(\Psi, \bar{\Phi})$  are observable.

Let  $f : \mathbb{C}^{Nn} \rightarrow \mathbb{C}^{Np}$  be a linear map such that, for  $x \in \mathbb{C}^{Nn}$ ,  $f(x) = \Psi x$ . Moreover, let  $\tau(\mu|\Phi)$  and  $\tau(\mu|\bar{\Phi})$  denote the geometric multiplicities of  $\mu$  for  $\Phi$  and  $\bar{\Phi}$ , respectively. In what follows, some conditions on the  $\Psi$ -discernibility of topological variations are derived in terms of the multiplicities of the eigenvalues.

**Corollary 1** Consider the networked system (2)-(3). A topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -discernible if and only if  $\dim\{f[S(\mu|\Phi)] + f[S(\mu|\bar{\Phi})]\} = \tau(\mu|\Phi) + \tau(\mu|\bar{\Phi})$ , for all  $\mu \in \sigma(\Phi) \cup \sigma(\bar{\Phi})$ .

**Proof:** Note that  $\dim\{f[S(\mu|\Phi)] + f[S(\mu|\bar{\Phi})]\} = \dim\{f[S(\mu|\Phi)]\} + \dim\{f[S(\mu|\bar{\Phi})]\} - \dim\{f[S(\mu|\Phi)] \cap f[S(\mu|\bar{\Phi})]\} = \dim\{S(\mu|\Phi)\} - \dim\{\mathcal{N}(\Psi) \cap S(\mu|\Phi)\} +$

$\mathbf{dim}\{S(\mu|\bar{\Phi})\} - \mathbf{dim}\{\mathcal{N}(\Psi) \cap S(\mu|\bar{\Phi})\} - \mathbf{dim}\{f[S(\mu|\Phi)] \cap f[S(\mu|\bar{\Phi})]\} = \tau(\mu|\Phi) + \tau(\mu|\bar{\Phi}) - \mathbf{dim}\{\mathcal{N}(\Psi) \cap S(\mu|\Phi)\} - \mathbf{dim}\{\mathcal{N}(\Psi) \cap S(\mu|\bar{\Phi})\} - \mathbf{dim}\{f[S(\mu|\Phi)] \cap f[S(\mu|\bar{\Phi})]\}$ . Thus,  $\mathbf{dim}\{f[S(\mu|\Phi)] + f[S(\mu|\bar{\Phi})]\} = \tau(\mu|\Phi) + \tau(\mu|\bar{\Phi})$  if and only if the following three conditions hold simultaneously:

$$\mathcal{N}(\Psi) \cap S(\mu|\Phi) = \{\mathbf{0}_{Nn}\}; \quad (6)$$

$$\mathcal{N}(\Psi) \cap S(\mu|\bar{\Phi}) = \{\mathbf{0}_{Nn}\}; \quad (7)$$

$$f[S(\mu|\Phi)] \cap f[S(\mu|\bar{\Phi})] = \{\mathbf{0}_{Nn}\}. \quad (8)$$

Necessity: If there exists  $\mu^* \in \sigma(\Phi) \cup \sigma(\bar{\Phi})$  such that  $\mathbf{dim}\{f[S(\mu^*|\Phi)] + f[S(\mu^*|\bar{\Phi})]\} < \tau(\mu^*|\Phi) + \tau(\mu^*|\bar{\Phi})$ , then at least one of (6), (7), (8) does not hold.

- Case 1: If (6) does not hold, then there exists a nonzero vector  $v \in S(\mu^*|\Phi)$  such that  $\Psi v = \mathbf{0}$ , which implies that  $\mathbf{0} \neq v \in \mathcal{N}(\Psi) \cap \{S(\mu^*|\Phi) \oplus S(\mu^*|\bar{\Phi})\}$ . Thus, it follows from Theorem 1 that the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible.
- Case 2: If (7) does not hold, one can similarly prove that the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible.
- Case 3: If (8) does not hold, then there exist  $v \in S(\mu^*|\Phi)$  and  $w \in S(\mu^*|\bar{\Phi})$  such that  $\Psi v = \Psi w \neq \mathbf{0}$ . If  $S(\mu^*|\Phi) \cap S(\mu^*|\bar{\Phi}) \neq \{\mathbf{0}_{Nn}\}$ , it follows from Theorem 1 that the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible; If  $S(\mu^*|\Phi) \cap S(\mu^*|\bar{\Phi}) = \{\mathbf{0}_{Nn}\}$ , then  $v - w \in \{S(\mu^*|\Phi) \oplus S(\mu^*|\bar{\Phi})\}$ . From  $\Psi v = \Psi w \neq \mathbf{0}$ , it follows that  $v \neq \mathbf{0}$  and  $w \neq \mathbf{0}$ . Then, one can show that  $v - w \neq \mathbf{0}$ . Since  $\Psi(v - w) = \mathbf{0}$ , one gets that  $\mathbf{0} \neq v - w \in \mathcal{N}(\Psi) \cap \{S(\mu^*|\Phi) \oplus S(\mu^*|\bar{\Phi})\}$ . According to Theorem 1, the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible.

Therefore, if there exists  $\mu^* \in \sigma(\Phi) \cup \sigma(\bar{\Phi})$  such that  $\mathbf{dim}\{f[S(\mu^*|\Phi)] + f[S(\mu^*|\bar{\Phi})]\} < \tau(\mu^*|\Phi) + \tau(\mu^*|\bar{\Phi})$ , then the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible.

Sufficiency: If the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible, then at least one condition in Theorem 1 does not hold.

- Case 1: There exists  $\mu^* \in \sigma(\Phi) \cup \sigma(\bar{\Phi})$  such that  $S(\mu^*|\Phi) \cap S(\mu^*|\bar{\Phi}) \neq \{\mathbf{0}_{Nn}\}$ . Then, there exists a nonzero vector  $x \in S(\mu^*|\Phi) \cap S(\mu^*|\bar{\Phi})$ . This will be discussed in the following two cases.
  - If  $x \notin \mathcal{N}(\Psi)$ , then  $\mathbf{0} \neq f(x) \in f[S(\mu^*|\Phi)] \cap f[S(\mu^*|\bar{\Phi})]$ , which indicates that (8) does not hold. Thus,  $\mathbf{dim}\{f[S(\mu^*|\Phi)] + f[S(\mu^*|\bar{\Phi})]\} < \tau(\mu^*|\Phi) + \tau(\mu^*|\bar{\Phi})$ .
  - If  $x \in \mathcal{N}(\Psi)$ , then  $\mathbf{0} \neq x \in \mathcal{N}(\Psi) \cap S(\mu^*|\Phi)$ , which implies that (6) does not hold. Thus,  $\mathbf{dim}\{f[S(\mu^*|\Phi)] + f[S(\mu^*|\bar{\Phi})]\} < \tau(\mu^*|\Phi) + \tau(\mu^*|\bar{\Phi})$ .
- Case 2: There exists  $\mu^* \in \sigma(\Phi) \cup \sigma(\bar{\Phi})$  such that  $\mathcal{N}(\Psi) \cap \{S(\mu^*|\Phi) \oplus S(\mu^*|\bar{\Phi})\} \neq \{\mathbf{0}_{Nn}\}$ . Then, there exists a nonzero vector  $x \in S(\mu^*|\Phi) \oplus S(\mu^*|\bar{\Phi})$  such that  $\Psi x = f(x) = \mathbf{0}$ . Let  $x = x_1 + x_2$ , where  $x_1 \in S(\mu^*|\Phi)$  and  $x_2 \in S(\mu^*|\bar{\Phi})$ . Then, it follows from

$f(x) = f(x_1 + x_2) = f(x_1) + f(x_2) = \mathbf{0}$  that  $f(x_1) = f(-x_2)$ . Note that  $x_1 \in S(\mu^*|\Phi)$  and  $-x_2 \in S(\mu^*|\bar{\Phi})$ . If  $f(x_1) = f(-x_2) \neq \mathbf{0}$ , then (8) does not hold. Thus,  $\mathbf{dim}\{f[S(\mu^*|\Phi)] + f[S(\mu^*|\bar{\Phi})]\} < \tau(\mu^*|\Phi) + \tau(\mu^*|\bar{\Phi})$ . In what follows, the case of  $f(x_1) = f(-x_2) = \mathbf{0}$  will be discussed. Noting that  $x = x_1 + x_2 \neq \mathbf{0}$ , one can verify that  $x_1$  and  $x_2$  are not both  $\mathbf{0}$ . If  $x_1 \neq \mathbf{0}$ , then  $\mathbf{0} \neq x_1 \in \mathcal{N}(\Psi) \cap S(\mu^*|\Phi)$ . Thus, (6) does not hold, which implies that  $\mathbf{dim}\{f[S(\mu^*|\Phi)] + f[S(\mu^*|\bar{\Phi})]\} < \tau(\mu^*|\Phi) + \tau(\mu^*|\bar{\Phi})$ . If  $x_1 = \mathbf{0}$ , then  $x_2 \neq \mathbf{0}$ . It follows that  $\mathbf{0} \neq -x_2 \in \mathcal{N}(\Psi) \cap S(\mu^*|\bar{\Phi})$ , which indicates that (7) does not hold. Thus,  $\mathbf{dim}\{f[S(\mu^*|\Phi)] + f[S(\mu^*|\bar{\Phi})]\} < \tau(\mu^*|\Phi) + \tau(\mu^*|\bar{\Phi})$ .

Therefore, if the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible, then there exists  $\mu^* \in \sigma(\Phi) \cup \sigma(\bar{\Phi})$  such that  $\mathbf{dim}\{f[S(\mu^*|\Phi)] + f[S(\mu^*|\bar{\Phi})]\} < \tau(\mu^*|\Phi) + \tau(\mu^*|\bar{\Phi})$ . ■

In what follows, a necessary condition on the  $\Psi$ -discernibility is derived, which is intuitive and easier to verify.

**Corollary 2** Consider the networked system (2)-(3). If the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -discernible, then  $Rank(\Delta) \times Rank(C) \geq \tau(\mu|\Phi) + \tau(\mu|\bar{\Phi})$ , for all  $\mu \in \sigma(\Phi) \cup \sigma(\bar{\Phi})$ .

**Proof:** Assume that there exists  $\mu^* \in \sigma(\Phi) \cup \sigma(\bar{\Phi})$  such that  $Rank(\Delta) \times Rank(C) < \tau(\mu^*|\Phi) + \tau(\mu^*|\bar{\Phi})$ . If  $S(\mu^*|\Phi) \cap S(\mu^*|\bar{\Phi}) \neq \{\mathbf{0}_{Nn}\}$ , it follows from Theorem 1 that the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible. If  $S(\mu^*|\Phi) \cap S(\mu^*|\bar{\Phi}) = \{\mathbf{0}_{Nn}\}$ , then  $\mathbf{dim}\{S(\mu^*|\Phi) \oplus S(\mu^*|\bar{\Phi})\} = \mathbf{dim}[S(\mu^*|\Phi)] + \mathbf{dim}[S(\mu^*|\bar{\Phi})] = \tau(\mu^*|\Phi) + \tau(\mu^*|\bar{\Phi}) > Rank(\Delta) \times Rank(C)$ . Noting that  $\mathbf{dim}\{\mathcal{N}(\Psi)\} + Rank(\Delta) \times Rank(C) = nN$ , it can be easily verified that  $\mathbf{dim}\{\mathcal{N}(\Psi)\} + \mathbf{dim}\{S(\mu^*|\Phi) \oplus S(\mu^*|\bar{\Phi})\} > Nn$ . It follows that  $\mathcal{N}(\Psi) \cap \{S(\mu^*|\Phi) \oplus S(\mu^*|\bar{\Phi})\} \neq \{\mathbf{0}_{Nn}\}$ . According to Theorem 1, the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible. Therefore, if there exists  $\mu^* \in \sigma(\Phi) \cup \sigma(\bar{\Phi})$  such that  $Rank(\Delta) \times Rank(C) < \tau(\mu^*|\Phi) + \tau(\mu^*|\bar{\Phi})$ , then the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible. ■

The effectiveness of the above corollary is illustrated via the following example.

**Example 1** Consider a simple network consisting of four connected identical nodes, shown in (a) of Fig. 1, with  $w_{21} = w_{32} = w_{43} = w_{14} = 1$ . Suppose that the output of the first node and the second node can be observed, i.e.,  $\delta_1 = \delta_2 = 1$ , with

$$A = \begin{bmatrix} 1 & 1 \\ 0 & 2 \end{bmatrix}, B = \begin{bmatrix} 1 & 0 \\ 0 & 0 \end{bmatrix}, C = \begin{bmatrix} 1 & 0 \end{bmatrix}.$$

![Figure 1: Network topologies. (a) G: A directed graph with four nodes labeled 1, 2, 3, 4. Node 1 is at the top left, 2 is at the bottom left, 3 is at the bottom right, and 4 is at the top right. There are directed edges: 1 to 2 (down), 2 to 3 (right), 3 to 4 (up), and 4 to 1 (left). (b) G-bar: The same graph as (a) but with the edge from node 4 to node 1 removed. The remaining edges are 1 to 2, 2 to 3, and 3 to 4.](547f726730e589392f239257a833ede3_img.jpg)

Figure 1: Network topologies. (a) G: A directed graph with four nodes labeled 1, 2, 3, 4. Node 1 is at the top left, 2 is at the bottom left, 3 is at the bottom right, and 4 is at the top right. There are directed edges: 1 to 2 (down), 2 to 3 (right), 3 to 4 (up), and 4 to 1 (left). (b) G-bar: The same graph as (a) but with the edge from node 4 to node 1 removed. The remaining edges are 1 to 2, 2 to 3, and 3 to 4.

Fig. 1. Network topologies

It can be easily verified that  $L = \begin{bmatrix} 0 & 0 & 0 & 1 \\ 1 & 0 & 0 & 0 \\ 0 & 1 & 0 & 0 \\ 0 & 0 & 1 & 0 \end{bmatrix}$  and  $\Delta =$

$\begin{bmatrix} 1 & 0 & 0 & 0 \\ 0 & 1 & 0 & 0 \end{bmatrix}$ . Assume that the edge from node 4 to node 1 is removed. Then, the new network is shown in (b) of

Fig. 1, with the topology matrix  $\bar{L} = \begin{bmatrix} 0 & 0 & 0 & 0 \\ 1 & 0 & 0 & 0 \\ 0 & 1 & 0 & 0 \\ 0 & 0 & 1 & 0 \end{bmatrix}$ .

It is easy to verify that  $\sigma(\Phi) = \{0, 2, 1+i, 1-i\}$  and  $\tau(2|\Phi) = 4$ . Moreover,  $\sigma(\bar{\Phi}) = \{1, 2\}$  and  $\tau(2|\bar{\Phi}) = 4$ . It follows that  $\tau(2|\Phi) + \tau(2|\bar{\Phi}) = 8$ . Thus,  $\text{Rank}(\Delta) \times \text{Rank}(C) = 2 < \tau(2|\Phi) + \tau(2|\bar{\Phi})$ . According to Corollary 2, the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible.

**Remark 3** It follows from Corollary 2 that to guarantee the  $\Psi$ -discernibility of the topological variation, the number of the sensors should not be less than  $\frac{\max_{\mu \in \sigma(\Phi) \cup \sigma(\bar{\Phi})} \tau(\mu|\Phi) + \tau(\mu|\bar{\Phi})}{\text{Rank}C}$ . Thus, Corollary 2 provides useful information on the number of the sensors required for the  $\Psi$ -discernibility.

## 4 Lower-dimensional conditions on the $\Psi$ -discernibility of topological changes

In Section 3, some eigenspace-based conditions on the  $\Psi$ -discernibility of topological changes are developed. In this section, some lower-dimensional conditions are further established by taking the network structures into consideration, which are easier to verify and apply.

Let  $\lambda_1, \lambda_2, \dots, \lambda_s$  be the eigenvalues of  $L$ . The Jordan chain of  $L$  associated with the eigenvalue  $\lambda_i$  is denoted as  $t_i^1, t_i^2, \dots, t_i^{\alpha_i}$ , where  $t_i^1$  is the top vector, and  $\alpha_i$  is the length of the Jordan chain. Moreover, denote the eigenvalues of  $A + \lambda_i H$  as  $\mu_i^j$ , with the corresponding

generalized Jordan chain about  $H$  denoted as  $\xi_{ij}^1, \xi_{ij}^2, \dots, \xi_{ij}^{\theta_{ij}}$ , where  $\xi_{ij}^1$  is the top vector, and  $\theta_{ij}$  is the length, for  $j = 1, \dots, p_i, i = 1, \dots, s$ . In [13], the eigenspaces of  $\Phi$  are expressed through the generalized eigenvectors of some matrices with lower dimensions.

**Lemma 1** [13] Let  $\lambda_1, \lambda_2, \dots, \lambda_s$  be the eigenvalues of  $L$ , and  $\mu_i^1, \dots, \mu_i^{p_i}$  be the eigenvalues of  $A + \lambda_i H$ , counting geometric multiplicities. Then,  $\mu_1^1, \dots, \mu_1^{p_1}, \dots, \mu_s^1, \dots, \mu_s^{p_s}$  are the eigenvalues of  $\Phi$ . Moreover, the eigenspace of  $\Phi$  corresponding to  $\mu_i^j$  is  $V_{ij} = \text{span}\{\eta_{ij}^1, \eta_{ij}^2, \dots, \eta_{ij}^{\gamma_{ij}}\}$ , where  $\eta_{ij}^1 = t_i^1 \otimes \xi_{ij}^1, \eta_{ij}^2 = t_i^2 \otimes \xi_{ij}^1 + t_i^1 \otimes \xi_{ij}^2, \dots, \eta_{ij}^{\gamma_{ij}} = t_i^{\gamma_{ij}} \otimes \xi_{ij}^1 + t_i^{\gamma_{ij}-1} \otimes \xi_{ij}^2 + \dots + t_i^1 \otimes \xi_{ij}^{\gamma_{ij}}, \gamma_{ij} = \min\{\alpha_i, \theta_{ij}\}, j = 1, \dots, p_i, i = 1, \dots, s$ . Let  $\Gamma(\mu) = \{(i, j) \in \mathbb{N} \times \mathbb{N} | \mu_i^j = \mu, 1 \leq j \leq p_i, 1 \leq i \leq s\}$ . Then,  $S(\mu|\Phi) = \bigoplus_{(i,j) \in \Gamma(\mu)} V_{ij}$ .

Similarly, the eigenspaces of  $\bar{\Phi}$  can be expressed through the generalized eigenvectors of some matrices with lower dimensions. Let  $\bar{\lambda}_1, \bar{\lambda}_2, \dots, \bar{\lambda}_{\bar{s}}$  be the eigenvalues of  $\bar{L}$ , and  $\bar{\mu}_i^1, \dots, \bar{\mu}_i^{\bar{p}_i}$  be the eigenvalues of  $A + \bar{\lambda}_i H$ , counting geometric multiplicities. Then,  $\bar{\mu}_1^1, \dots, \bar{\mu}_1^{\bar{p}_1}, \dots, \bar{\mu}_{\bar{s}}^1, \dots, \bar{\mu}_{\bar{s}}^{\bar{p}_{\bar{s}}}$  are the eigenvalues of  $\bar{\Phi}$ . Moreover, the eigenspace of  $\bar{\Phi}$  corresponding to  $\bar{\mu}_i^j$  is  $\bar{V}_{ij} = \text{span}\{\bar{\eta}_{ij}^1, \bar{\eta}_{ij}^2, \dots, \bar{\eta}_{ij}^{\bar{\gamma}_{ij}}\}$ , where  $\bar{\eta}_{ij}^1, \dots, \bar{\eta}_{ij}^{\bar{\gamma}_{ij}}$  can be expressed using the generalized eigenvectors of  $\bar{L}$  and  $A + \bar{\lambda}_i H$  as shown in Lemma 1,  $j = 1, \dots, \bar{p}_i, i = 1, \dots, \bar{s}$ . Let  $\bar{\Gamma}(\mu) = \{(i, j) \in \mathbb{N} \times \mathbb{N} | \bar{\mu}_i^j = \mu, 1 \leq j \leq \bar{p}_i, 1 \leq i \leq \bar{s}\}$ . Then,  $S(\mu|\bar{\Phi}) = \bigoplus_{(i,j) \in \bar{\Gamma}(\mu)} \bar{V}_{ij}$ .

Next, a necessary and sufficient condition on the  $\Psi$ -discernibility of topological changes is established.

**Theorem 2** Consider the networked system (2)-(3). A topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -discernible if and only if for all  $\mu \in \sigma(\Phi) \cup \sigma(\bar{\Phi})$ , the following two conditions hold simultaneously:

$$(1) \left\{ \bigoplus_{(i,j) \in \Gamma(\mu)} V_{ij} \right\} \cap \left\{ \bigoplus_{(i,j) \in \bar{\Gamma}(\mu)} \bar{V}_{ij} \right\} = \{O_{Nn}\};$$

$$(2) \mathcal{N}(\Delta \otimes C) \cap \left\{ \left\{ \bigoplus_{(i,j) \in \Gamma(\mu)} V_{ij} \right\} \oplus \left\{ \bigoplus_{(i,j) \in \bar{\Gamma}(\mu)} \bar{V}_{ij} \right\} \right\} = \{O_{Nn}\}.$$

**Proof:** The proof follows from Theorem 1 and Lemma 1 directly, thus is omitted. ■

The effectiveness of Theorem 2 is demonstrated by the following example.

**Example 2** Consider a simple network of three connected identical nodes, shown in (a) of Fig. 2, with  $w_{21} =$

$w_{32} = w_{23} = 1$ . Suppose that the output of the first node and the third node can be observed, i.e.,  $\delta_1 = \delta_3 = 1$ , with

$$A = \begin{bmatrix} 1 & 0 \\ 1 & 1 \end{bmatrix}, H = \begin{bmatrix} 1 & 0 \\ 0 & 1 \end{bmatrix}, C = \begin{bmatrix} 1 & 0 \\ 0 & 1 \end{bmatrix}.$$

It can be easily verified that  $L = \begin{bmatrix} 0 & 0 & 0 \\ 1 & 0 & 1 \\ 0 & 1 & 0 \end{bmatrix}$  and  $\Delta =$

$\begin{bmatrix} 1 & 0 & 0 \\ 0 & 0 & 1 \end{bmatrix}$ . The eigenvalues of  $L$  are  $\lambda_1 = 0$ ,  $\lambda_2 = 1$  and  $\lambda_3 = -1$ , with the corresponding eigenvectors  $t_1 = e_1 - e_3$ ,  $t_2 = e_2 + e_3$  and  $t_3 = e_2 - e_3$ , respectively. Then, the eigenvalue of  $A + \lambda_1 H = A$  is  $\mu_1^1 = 1$ , with the corresponding eigenvector  $\xi_{11}^1 = e_2$ , and the eigenvalue of  $A + \lambda_2 H = A + H$  is  $\mu_2^1 = 2$ , with the corresponding eigenvector  $\xi_{21}^1 = e_2$ . The eigenvalue of  $A + \lambda_3 H = A - H$  is  $\mu_3^1 = 0$ , with the corresponding eigenvector  $\xi_{31}^1 = e_2$ . Thus, it follows that  $S(1|\Phi) = V_{11} = \text{span}\{(e_1 - e_3) \otimes e_2\}$ ,  $S(2|\Phi) = V_{21} = \text{span}\{(e_2 + e_3) \otimes e_2\}$ , and  $S(0|\Phi) = V_{31} = \text{span}\{(e_2 - e_3) \otimes e_2\}$ .

Assume that the edge from node 3 to node 2 is removed.

Then, the new topology matrix is  $\bar{L} = \begin{bmatrix} 0 & 0 & 0 \\ 1 & 0 & 0 \\ 0 & 1 & 0 \end{bmatrix}$ . The

eigenvalue of  $\bar{L}$  is  $\bar{\lambda}_1 = 0$ , with the corresponding Jordan chain  $\bar{t}_1^1 = e_3$ ,  $\bar{t}_1^2 = e_2$ ,  $\bar{t}_1^3 = e_1$ . So, the eigenvalue of  $A + \bar{\lambda}_1 H = A$  is  $\bar{\mu}_1^1 = 1$ , with the generalized Jordan chain  $\bar{\xi}_{11}^1 = e_2$ ,  $\bar{\xi}_{11}^2 = -e_1$ . Thus, it follows that  $S(1|\bar{\Phi}) = \bar{V}_{11} = \text{span}\{e_3 \otimes e_2, e_2 \otimes e_2 + e_3 \otimes (-e_1)\}$ .

Noting that  $S(0|\bar{\Phi}) = S(2|\bar{\Phi}) = \{\mathbf{0}\}$ , one can easily verify that  $S(0|\Phi) \cap S(0|\bar{\Phi}) = S(2|\Phi) \cap S(2|\bar{\Phi}) = S(1|\Phi) \cap S(1|\bar{\Phi}) = \{\mathbf{0}\}$ . Moreover,  $\mathcal{N}(\Delta \otimes C) \cap S(0|\Phi) = \mathcal{N}(\Delta \otimes C) \cap S(2|\Phi) = \mathcal{N}(\Delta \otimes C) \cap \{S(1|\Phi) \oplus S(1|\bar{\Phi})\} = \{\mathbf{0}\}$ . Therefore, it follows from Theorem 2 that the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -discernible.

![Figure 2 shows two network topologies. (a) G is a directed graph with three nodes: 1, 2, and 3. Node 1 has a self-loop and a directed edge to node 2. Node 2 has a self-loop and a directed edge to node 3. Node 3 has a self-loop. (b) G-bar is a directed graph with three nodes: 1, 2, and 3. Node 1 has a self-loop and a directed edge to node 2. Node 2 has a self-loop and a directed edge to node 3. Node 3 has a self-loop. The difference is that in (b), the edge from node 3 to node 2 is removed.](9791722d75115ddcc599b07d7bc35d73_img.jpg)

Figure 2 shows two network topologies. (a) G is a directed graph with three nodes: 1, 2, and 3. Node 1 has a self-loop and a directed edge to node 2. Node 2 has a self-loop and a directed edge to node 3. Node 3 has a self-loop. (b) G-bar is a directed graph with three nodes: 1, 2, and 3. Node 1 has a self-loop and a directed edge to node 2. Node 2 has a self-loop and a directed edge to node 3. Node 3 has a self-loop. The difference is that in (b), the edge from node 3 to node 2 is removed.

Fig. 2. Network topologies

In the following, some lower-dimensional and easily-verified conditions on the  $\Psi$ -discernibility of topological variations are presented.

**Corollary 3** If a topological change  $L \rightarrow \bar{L}$  for the networked system (2)-(3) is  $\Psi$ -discernible, then the topolog-

ical change  $L \rightarrow \bar{L}$  is  $\Delta$ -discernible for the system

$$\begin{cases} \dot{x} = Lx, \\ y = \Delta x. \end{cases} \quad (9)$$

**Proof:** Let  $\lambda$  be an eigenvalue of  $L$ , with the corresponding eigenspace  $S(\lambda|L)$ . Moreover, let  $\bar{\lambda}$  be an eigenvalue of  $\bar{L}$ , with the associated eigenspace  $S(\bar{\lambda}|\bar{L})$ . If the topological change  $L \rightarrow \bar{L}$  is  $\Delta$ -indiscernible for system (9), then at least one of the following cases occurs.

- There exists  $\lambda^* \in \sigma(L) \cup \sigma(\bar{L})$  such that  $S(\lambda^*|L) \cap S(\lambda^*|\bar{L}) \neq \{\mathbf{0}_N\}$ . Then, there exists a nonzero vector  $t \in S(\lambda^*|L) \cap S(\lambda^*|\bar{L})$ . Let  $\mu^*$  be an eigenvalue of  $A + \lambda^*H$  with the corresponding eigenvector  $\xi$ . According to Lemma 1, one has

$$\mathbf{0}_{Nn} \neq t \otimes \xi \in \left\{ \bigoplus_{(i,j) \in \Gamma(\mu^*)} V_{ij} \right\} \cap \left\{ \bigoplus_{(i,j) \in \bar{\Gamma}(\mu^*)} \bar{V}_{ij} \right\}.$$

Thus, it follows from Theorem 2 that the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible for the networked system (2)-(3).

- For all  $\lambda \in \sigma(L) \cup \sigma(\bar{L})$ ,  $S(\lambda|L) \cap S(\lambda|\bar{L}) = \{\mathbf{0}_N\}$ ; But there exists  $\lambda^* \in \sigma(L) \cup \sigma(\bar{L})$  such that  $\mathcal{N}(\Delta) \cap \{S(\lambda^*|L) \oplus S(\lambda^*|\bar{L})\} \neq \{\mathbf{0}_N\}$ . It follows that there exists a nonzero vector  $t \in \{S(\lambda^*|L) \oplus S(\lambda^*|\bar{L})\}$  such that  $\Delta t = \mathbf{0}$ . Let  $t = t_1 + t_2$ , where  $t_1 \in S(\lambda^*|L)$  and  $t_2 \in S(\lambda^*|\bar{L})$ . Moreover, let  $\mu^*$  be an eigenvalue of  $A + \lambda^*H$ , with the corresponding eigenvector  $\xi$ . It follows from Lemma 1 that  $t_1 \otimes \xi \in \bigoplus_{(i,j) \in \Gamma(\mu^*)} V_{ij}$  and  $t_2 \otimes \xi \in \bigoplus_{(i,j) \in \bar{\Gamma}(\mu^*)} \bar{V}_{ij}$ . If

$$\left\{ \bigoplus_{(i,j) \in \Gamma(\mu^*)} V_{ij} \right\} \cap \left\{ \bigoplus_{(i,j) \in \bar{\Gamma}(\mu^*)} \bar{V}_{ij} \right\} \neq \{\mathbf{0}_{Nn}\},$$

it follows from Theorem 2 that the topological change  $L \rightarrow \bar{L}$  for the networked system (2)-(3) is  $\Psi$ -indiscernible.

$$\text{If } \left\{ \bigoplus_{(i,j) \in \Gamma(\mu^*)} V_{ij} \right\} \cap \left\{ \bigoplus_{(i,j) \in \bar{\Gamma}(\mu^*)} \bar{V}_{ij} \right\} = \{\mathbf{0}_{Nn}\}, \text{ then}$$

$$t_1 \otimes \xi + t_2 \otimes \xi \in \left\{ \bigoplus_{(i,j) \in \Gamma(\mu^*)} V_{ij} \right\} \oplus \left\{ \bigoplus_{(i,j) \in \bar{\Gamma}(\mu^*)} \bar{V}_{ij} \right\}.$$

Since  $t \neq \mathbf{0}$  and  $\xi \neq \mathbf{0}$ , it follows that  $t_1 \otimes \xi + t_2 \otimes \xi = t \otimes \xi \neq \mathbf{0}$ . Noting that  $\Delta t = \mathbf{0}$ , one gets that  $(\Delta \otimes C)(t_1 \otimes \xi + t_2 \otimes \xi) = (\Delta \otimes C)(t \otimes \xi) = (\Delta t) \otimes (C\xi) = \mathbf{0}$ . Thus,  $\mathbf{0} \neq t_1 \otimes \xi + t_2 \otimes \xi$

$$\in \mathcal{N}(\Delta \otimes C) \cap \left\{ \left\{ \bigoplus_{(i,j) \in \Gamma(\mu^*)} V_{ij} \right\} \oplus \left\{ \bigoplus_{(i,j) \in \bar{\Gamma}(\mu^*)} \bar{V}_{ij} \right\} \right\},$$

$$\text{which indicates that } \mathcal{N}(\Delta \otimes C) \cap \left\{ \bigoplus_{(i,j) \in \Gamma(\mu^*)} V_{ij} \right\} \oplus$$

$$\left\{ \bigoplus_{(i,j) \in \bar{\Gamma}(\mu^*)} \bar{V}_{ij} \right\} \neq \{\mathbf{0}_{Nn}\}. \text{ According to Theorem}$$

2, the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible for the networked system (2)-(3).

Therefore, if the topological change  $L \rightarrow \bar{L}$  is  $\Delta$ -

indiscernible for system (9), then this topological change is  $\Psi$ -indiscernible for the whole networked system (2)-(3). ■

The effectiveness of Corollary 3 is demonstrated by the following example.

**Example 3** Consider a simple network of three connected identical nodes, shown in (a) of Fig. 2, with  $w_{21} = w_{32} = w_{23} = 1$ . Suppose that the output of the second node and the third node can be observed, i.e.,  $\delta_2 = \delta_3 = 1$ , with

$$A = \begin{bmatrix} 1 & 0 \\ 1 & 1 \end{bmatrix}, H = \begin{bmatrix} 0 & 0 \\ 0 & 1 \end{bmatrix}, C = \begin{bmatrix} 1 & 0 \\ 0 & 1 \end{bmatrix}.$$

It can be easily verified that  $L = \begin{bmatrix} 0 & 0 & 0 \\ 1 & 0 & 1 \\ 0 & 1 & 0 \end{bmatrix}$  and  $\Delta =$

$\begin{bmatrix} 0 & 1 & 0 \\ 0 & 0 & 1 \end{bmatrix}$ . Assume that the edge from node 3 to node 2 is removed. Then, the new topology matrix is  $\bar{L} = \begin{bmatrix} 0 & 0 & 0 \\ 1 & 0 & 0 \\ 0 & 1 & 0 \end{bmatrix}$ .

It is easy to verify that  $S(0|L) = \text{span}\{e_1 - e_3\}$  and  $S(0|\bar{L}) = \text{span}\{e_3\}$ . It follows that  $\mathcal{N}(\Delta) \cap \{S(0|L) \oplus S(0|\bar{L})\} \neq \{0_3\}$ , which implies that the topological change  $L \rightarrow \bar{L}$  is  $\Delta$ -indiscernible for system (9). Therefore, it follows from Corollary 3 that the topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -indiscernible for the whole networked system.

Actually, one cannot detect the topological change from the output trajectories when there exists some pair of initial states  $(X_0, \bar{X}_0)$  satisfying  $\Psi e^{\Phi t} X_0 = \Psi e^{\bar{\Phi} t} \bar{X}_0$ . An example is  $X_0 = [0, 1, 0, 0, 0, -1]^T$  and  $\bar{X}_0 = [0, 0, 0, 0, 0, 1]^T$ , with which the two networked systems will generate exactly the same output trajectories.

The above corollary reveals that the network topologies and the sensor locations have significant effect on the  $\Psi$ -discernibility for the whole networked system. In the following, the effect of the inner interactions, the node-system dynamics and output on the  $\Psi$ -discernibility is discussed.

**Corollary 4** Consider the networked system (2)-(3). If a topological change  $L \rightarrow \bar{L}$  is  $\Psi$ -discernible for the networked system (2)-(3), then the following two conditions hold simultaneously:

- (1)  $(A, H)$  is observable;
- (2)  $(A + \lambda H, C)$  is observable, for all  $\lambda \in \{\sigma(L) \cup \sigma(\bar{L})\}$ ;

**Proof:** Using Theorem 2, it can be proved easily. Thus, the proof is omitted. ■

**Remark 4** The possibility of detecting topological variations for networked LTI systems by observing network states has been investigated in [13], with some lower-dimensional discernibility conditions established. However, complete observation of the full network states is unrealistic in practical applications. In most situations, only partial information about the node-system state is accessible, and only a subset of nodes are available for measurement. In this paper, the results in [13] are generalized to the case of detecting topological changes by observing output trajectories. Thus, the new conditions here are more general and have broader applicability in practice. Compared with the conditions given in [4] and [18], which require the network topology to be undirected, the new conditions remove this requirement.

**Remark 5** The  $\Psi$ -discernibility of topological variations has been analyzed through eigenanalysis of the original and the modified networks in Section 3. However, for most large-scale networked systems with higher-dimensional node-systems, this method cannot be applied efficiently. In this section, some lower-dimensional conditions on the  $\Psi$ -discernibility of topological changes are established. Only the properties of some smaller matrices are required to be checked. These lower-dimensional conditions allow to check the  $\Psi$ -discernibility much more easily, which also reveal how the topological variations, sensor locations, node-system dynamics and output, as well as inner interactions altogether affect the  $\Psi$ -discernibility of the topological variation.

## 5 Output discernibility of topological variations for multi-agent systems

In this section, the output discernibility of topological variations for multi-agent systems is revisited.

### 5.1 Problem statement

Consider a multi-agent system consisting of  $N$  agents as follows:

$$\begin{cases} \dot{x}_i = Ax_i + Bu_i, \\ y_i = Cx_i, \end{cases} \quad i = 1, 2, \dots, N,$$

where  $x_i \in \mathbb{R}^n$ ,  $u_i \in \mathbb{R}^p$  and  $y_i \in \mathbb{R}^m$  are the state, the input and the output of the  $i$ th agent, respectively;  $A \in \mathbb{R}^{n \times n}$ ,  $B \in \mathbb{R}^{n \times p}$  and  $C \in \mathbb{R}^{m \times n}$  are the state matrix, the input matrix and the output matrix, respectively.

Agent  $i$  is a neighbor of agent  $j$  if its state is known by agent  $j$ . Here, assume that the neighboring relationships are fixed, which can be described by an undirected and weighted graph  $\mathcal{G} = (\mathcal{V}, \mathcal{E}, \mathcal{W})$ . The coupling input  $u_i$  to agent  $i$  is determined by the diffusive coupling rule based on the neighboring relations as follows:

$$u_i = \sum_{j \in N_i} w_{ij}(x_j - x_i),$$

where  $w_{ij} > 0$  with  $w_{ij} = w_{ji}$  for  $(i, j) \in \mathcal{E}$ , and  $N_i$  denotes the neighbor set of node  $i$ .

Let  $\mathcal{L} = [l_{ij}] \in \mathbb{R}^{N \times N}$  be the graph Laplacian induced by  $\mathcal{G}$ , with

$$l_{ij} := \begin{cases} \sum_{k \in N_i} w_{ik}, & j = i; \\ -w_{ij}, & j \neq i. \end{cases}$$

Moreover, let  $\mathcal{Q} \subset \mathcal{V}$  denote the subset of nodes whose output is available for measurement, and  $\Delta = \text{col}(e_i^T, i \in \mathcal{Q}) \in \mathbb{R}^{|\mathcal{Q}| \times N}$ . Let  $X = [x_1^T, x_2^T, \dots, x_N^T]^T$  and  $Y = \text{col}(y_i, i \in \mathcal{Q})$  be the state and the output of the whole multi-agent system, respectively. Then, the multi-agent system can be rewritten in a compact form as

$$\begin{cases} \dot{X} = FX, \\ Y = MX, \end{cases} \quad (10)$$

with

$$F = I_N \otimes A - \mathcal{L} \otimes B, \quad M = \Delta \otimes C. \quad (11)$$

The nominal multi-agent system is represented by  $(\mathcal{G}, F, M)$ . Here, consider a variation in the network structure, thereafter the new multi-agent system is described by

$$\begin{cases} \dot{\bar{X}} = \bar{F}\bar{X}, \\ \bar{Y} = M\bar{X}, \end{cases} \quad (12)$$

with

$$\bar{F} = I_N \otimes A - \bar{\mathcal{L}} \otimes B, \quad (13)$$

where  $\bar{\mathcal{L}}$  is the graph Laplacian induced by the new topology  $\bar{\mathcal{G}}$ . Denote the new multi-agent system by  $(\bar{\mathcal{G}}, \bar{F}, M)$ . Before moving on, the definitions of indiscernible initial state and always-discernible topological change are reviewed.

**Definition 5** [18] Consider the multi-agent system (10)-(11). An initial state  $X_0 \in \mathbb{R}^{Nn}$  is called indiscernible with respect to the topological change  $\mathcal{L} \rightarrow \bar{\mathcal{L}}$  if and only if

$$X(t) = e^{Ft}X_0 = e^{\bar{F}t}X_0 = \bar{X}(t), \quad \forall t \geq 0.$$

**Definition 6** [18] For the multi-agent system (10)-(11), a topological change  $\mathcal{L} \rightarrow \bar{\mathcal{L}}$  is called always-discernible if there is no (nontrivial) indiscernible initial state. Otherwise, the topological change is called possibly-indiscernible.

For the multi-agent systems  $(\mathcal{G}, F, M)$  and  $(\bar{\mathcal{G}}, \bar{F}, M)$ , let the set of all the real  $M$ -indiscernible pairs of initial states be denoted as  $T_M(F, \bar{F}) = \{(X_0, \bar{X}_0) \in \mathbb{R}^{Nn} \times \mathbb{R}^{Nn} | Me^{Ft}X_0 = Me^{\bar{F}t}\bar{X}_0, \forall t \geq 0\}$ . Moreover, let  $T_I(F, \bar{F}) = \{(X_0, X_0) \in \mathbb{R}^{Nn} \times \mathbb{R}^{Nn} | e^{Ft}X_0 = e^{\bar{F}t}X_0, \forall t \geq 0\}$ . Noting that an indiscernible initial state always generates indiscernible output trajectories, one has  $T_I(F, \bar{F}) \subseteq T_M(F, \bar{F})$ . In what follows, the conditions under which the output matrix  $M$  guarantees  $T_M(F, \bar{F}) = T_I(F, \bar{F})$  will be investigated. If  $T_M(F, \bar{F}) = T_I(F, \bar{F})$ , the matrix  $M$  is said to ensure the output discernibility of the multi-agent systems  $(\mathcal{G}, F, M)$  and  $(\bar{\mathcal{G}}, \bar{F}, M)$ .

### 5.2 A counterexample and a new condition on output discernibility

Recall a condition on the output discernibility of the multi-agent systems  $(\mathcal{G}, F, M)$  and  $(\bar{\mathcal{G}}, \bar{F}, M)$  proposed in [4], copied as follows:

**Theorem 3** [4] The matrix  $M$  ensures the output discernibility of the multi-agent systems  $(\mathcal{G}, F, M)$  and  $(\bar{\mathcal{G}}, \bar{F}, M)$  if and only if the following conditions hold simultaneously:

- (1) the pair  $(\mathcal{L}, \Delta)$  is observable;
- (2) the pair  $(\bar{\mathcal{L}}, \Delta)$  is observable;
- (3) the pair  $(A - \lambda B, C)$  is observable for every  $\lambda \in \sigma(\mathcal{L}) \cup \sigma(\bar{\mathcal{L}})$ ;
- (4) the matrix  $\Delta$  ensures the output discernibility of the two systems with state matrices  $\mathcal{L}$  and  $\bar{\mathcal{L}}$ ;
- (5) the matrix  $C$  ensures the output discernibility of the two systems with state matrices  $A - \lambda B$  and  $A - \bar{\lambda} B$ , for  $\lambda \in \sigma(\mathcal{L})$  and  $\bar{\lambda} \in \sigma(\bar{\mathcal{L}})$  with the corresponding eigenvectors  $t$  and  $\bar{t}$ , respectively, which satisfy  $\Delta t = \Delta \bar{t}$ .

In this theorem, it was claimed that the conditions are necessary and sufficient. However, the following counterexample shows that the conditions may not be sufficient.

**Example 4** Consider a multi-agent system consisting of three connected identical agents, shown in (a) of Fig. 3, with  $w_{12} = w_{21} = w_{23} = w_{32} = w_{13} = w_{31} = 1$ . Suppose that the output of the first node and the third node can be observed, i.e.,  $\mathcal{Q} = \{1, 3\}$ , with

$$A = \begin{bmatrix} 1 & 1 \\ 0 & 2 \end{bmatrix}, \quad B = \begin{bmatrix} 0 & 0 \\ 0 & 1 \end{bmatrix}, \quad C = \begin{bmatrix} 1 & 0 \end{bmatrix}.$$

![Figure 3 shows two network topologies. (a) G is a triangle with nodes 1, 2, and 3. (b) G-bar is a path graph with nodes 1, 2, and 3, where the edge between nodes 1 and 3 has been removed.](a5ee5c23b6dc52ec1d724b76d5a5f58f_img.jpg)

Figure 3 shows two network topologies. (a) G is a triangle with nodes 1, 2, and 3. (b) G-bar is a path graph with nodes 1, 2, and 3, where the edge between nodes 1 and 3 has been removed.

Fig. 3. Network topologies

It can be easily verified that  $\mathcal{L} = \begin{bmatrix} 2 & -1 & -1 \\ -1 & 2 & -1 \\ -1 & -1 & 2 \end{bmatrix}$  and

$\Delta = \begin{bmatrix} 1 & 0 & 0 \\ 0 & 0 & 1 \end{bmatrix}$ . Assume that the edge connecting nodes 3 and 1 is removed. Then, the new network is shown in (b) of Fig. 3, with the Laplacian matrix  $\bar{\mathcal{L}} = \begin{bmatrix} 1 & -1 & 0 \\ -1 & 2 & -1 \\ 0 & -1 & 1 \end{bmatrix}$ .

It is easy to verify that both  $(\mathcal{L}, \Delta)$  and  $(\bar{\mathcal{L}}, \Delta)$  are observable. The eigenvalues of  $\mathcal{L}$  are  $\lambda_1 = 0$ ,  $\lambda_2 = 1$  and  $\lambda_3 = 3$ . Moreover, the eigenvalues of  $\bar{\mathcal{L}}$  are  $\bar{\lambda}_1 = 0$  and  $\bar{\lambda}_2 = 3$ . One can easily verify that  $(A - \lambda B, C)$  is observable, for  $\lambda = 0, 1, 3$ .

Next, condition (4) will be checked. The eigenspaces of  $\mathcal{L}$  are  $S(0|\mathcal{L}) = \text{span}\{e_1 + e_2 + e_3\}$ ,  $S(1|\mathcal{L}) = \text{span}\{-e_1 + e_3\}$  and  $S(3|\mathcal{L}) = \text{span}\{e_1 - 2e_2 + e_3\}$ . Moreover, the eigenspaces of  $\bar{\mathcal{L}}$  are  $S(0|\bar{\mathcal{L}}) = \text{span}\{e_1 + e_2 + e_3\}$  and  $S(3|\bar{\mathcal{L}}) = \text{span}\{-e_1 + e_2, -e_1 + e_3\}$ . One can easily verify that  $T_\Delta(\mathcal{L}, \bar{\mathcal{L}}) = \{(X_0, X_0) | X_0 \in \text{span}\{e_1 + e_2 + e_3, e_1 - 2e_2 + e_3\}\}$ . Moreover, it is obvious that  $\mathcal{L}$  and  $\bar{\mathcal{L}}$  have two common eigenpairs, which are  $(0, e_1 + e_2 + e_3)$  and  $(3, e_1 - 2e_2 + e_3)$ . From the results given in [18], it can be verified that  $T_I(\mathcal{L}, \bar{\mathcal{L}}) = \{(X_0, X_0) | X_0 \in \text{span}\{e_1 + e_2 + e_3, e_1 - 2e_2 + e_3\}\} = T_\Delta(\mathcal{L}, \bar{\mathcal{L}})$ . Thus, matrix  $\Delta$  ensures the output discernibility of the two systems with state matrices  $\mathcal{L}$  and  $\bar{\mathcal{L}}$ .

Finally, condition (5) will be checked. Since  $t = e_1 - 2e_2 + e_3 \in S(3|\mathcal{L})$  and  $\bar{t} = e_1 + e_2 + e_3 \in S(0|\bar{\mathcal{L}})$  satisfy that  $\Delta t = \Delta \bar{t}$ , one needs to check whether matrix  $C$  can ensure the output discernibility of the two systems with state matrices  $A - 3B$  and  $A$ . The eigenspaces of  $A - 3B$  are  $S(1|A - 3B) = \text{span}\{e_1\}$  and  $S(-1|A - 3B) = \text{span}\{e_1 - 2e_2\}$ . Moreover, the eigenspaces of  $A$  are  $S(1|A) = \text{span}\{e_1\}$  and  $S(2|A) = \text{span}\{e_1 + e_2\}$ . It is easy to verify that  $T_C(A - 3B, A) = \{(X_0, X_0) | X_0 \in \text{span}\{e_1\}\} =$

$T_I(A - 3B, A)$ . Thus, matrix  $C$  ensures the output discernibility of the two systems with state matrices  $A - 3B$  and  $A$ . Also, since  $t = -e_1 + e_3 \in S(1|\mathcal{L})$  and  $\bar{t} = -e_1 + e_3 \in S(3|\bar{\mathcal{L}})$  satisfy that  $\Delta t = \Delta \bar{t}$ , one needs to check whether matrix  $C$  can ensure the output discernibility of the two systems with state matrices  $A - B$  and  $A - 3B$ . Note that the eigenspace of  $A - B$  is  $S(1|A - B) = \text{span}\{e_1\}$ . It is easy to verify that  $T_C(A - B, A - 3B) = \{(X_0, X_0) | X_0 \in \text{span}\{e_1\}\} = T_I(A - B, A - 3B)$ . Thus, matrix  $C$  ensures the output discernibility of the two systems with state matrices  $A - B$  and  $A - 3B$ . Therefore, condition (5) holds.

From the results given in [4], it would be deduced that  $M$  ensures the output discernibility of the multi-agent systems  $(\mathcal{G}, F, M)$  and  $(\bar{\mathcal{G}}, \bar{F}, M)$ . However, if one chooses  $(X_0, \bar{X}_0) = ([0, 0, 3, 0, 0, 0]^T, \mathbf{0}_6) \notin T_I(F, \bar{F})$ , the two multi-agent systems generate exactly the same output trajectories. Thus,  $T_M(F, \bar{F}) \neq T_I(F, \bar{F})$ , which indicates that  $M$  actually does not ensure the output discernibility. Therefore, the sufficiency of the condition given in [4] does not hold.

Note that Theorem 1 can also be used to verify the  $M$ -discernibility of the topological variation  $\mathcal{L} \rightarrow \bar{\mathcal{L}}$  for the multi-agent system (10)-(11). Moreover, a topological variation is  $M$ -discernible if and only if the topological variation is always-discernible and matrix  $M$  ensures the output discernibility. The first condition in Theorem 1 requires that the topological change  $\mathcal{L} \rightarrow \bar{\mathcal{L}}$  is always-discernible, while the second one requires that matrix  $M$  ensures the output discernibility. Based on the results in Section 3, a new necessary and sufficient condition on the output discernibility of the multi-agent systems  $(\mathcal{G}, F, M)$  and  $(\bar{\mathcal{G}}, \bar{F}, M)$  can be established as follows.

Let  $\phi$  be an eigenvalue of  $F$ , with the corresponding eigenspace  $S(\phi|F)$ . Moreover, let  $\bar{\phi}$  be an eigenvalue of  $\bar{F}$ , with the associated eigenspace  $S(\bar{\phi}|\bar{F})$ . Now, a condition on the output discernibility of the multi-agent systems  $(\mathcal{G}, F, M)$  and  $(\bar{\mathcal{G}}, \bar{F}, M)$  is presented.

**Corollary 5** The matrix  $M$  ensures the output discernibility of the multi-agent systems  $(\mathcal{G}, F, M)$  and  $(\bar{\mathcal{G}}, \bar{F}, M)$  if and only if for all  $\phi \in \sigma(F) \cup \sigma(\bar{F})$ , one has

$$\mathcal{N}(M) \cap \{S(\phi|F) + S(\phi|\bar{F})\} = \{\mathbf{0}_{Nn}\}.$$

**Remark 6** Some conditions on the output discernibility of topological variations for undirected networks have been established in [4]. The new condition proposed in Corollary 5 can handle both undirected and directed networks, thus is more general.

By using Lemma 1, the eigenspaces of  $F$  and  $\bar{F}$  can be expressed through the generalized eigenvectors of some matrices with lower dimensions. Similarly as in Section 4, some lower-dimensional conditions on the out-

put discernibility of the multi-agent systems  $(\mathcal{G}, F, M)$  and  $(\bar{\mathcal{G}}, \bar{F}, M)$  can also be established.

Finally, the effectiveness of the new condition can also be illustrated by Example 4.

It suffices to observe that  $S(1|F) = \text{span}\{(e_1 + e_2 + e_3) \otimes e_1, (-e_1 + e_3) \otimes e_1, (e_1 - 2e_2 + e_3) \otimes e_1\}$ . There exists  $\eta = [0, 0, 3, 0, 0, 0]^T = [0, 0, 3, 0, 0, 0]^T + \mathbf{0}_6 \in S(1|F) + S(1|\bar{F})$  such that  $M\eta = 0$ . Thus,  $\mathcal{N}(M) \cap \{S(1|F) + S(1|\bar{F})\} \neq \{\mathbf{0}_{Nn}\}$ . From Corollary 5, it follows that the matrix  $M$  does not ensure the output discernibility of the multi-agent systems  $(\mathcal{G}, F, M)$  and  $(\bar{\mathcal{G}}, \bar{F}, M)$ .

## 6 Conclusions

This paper has investigated the conditions under which a topological variation in networked LTI systems can be detected by observing output trajectories. The considered network topology can be general, directed and weighted. A necessary and sufficient condition on the  $\Psi$ -discernibility of topological variations has been established in terms of the eigenspaces of the original and the modified networks. Further, by taking the network structures into account, some lower-dimensional conditions on the  $\Psi$ -discernibility have been derived. The conditions have generalized the results given in [13], assuming only partial state variables of the networks are available for measurement. Moreover, the output discernibility of topological variations for multi-agent systems has been revisited. It is found that the sufficiency of the criterion given in [4] does not hold. Consequently, a complete necessary and sufficient condition is established. In future studies, the important issue of restoring the discernibility of topological variations for networked LTI systems will be investigated.

## References

- [1] Albert, R., Jeong, H., & Barabasi, A. L. (2000). Error and attack tolerance of complex networks. *Nature*, 406(6794), 378–382.
- [2] Banavar, J., Colaiori, F., Flammini, A., Maritan, A., & Rinaldo, A. (2000). Topology of the fittest transportation network. *Phys. Rev. Lett.*, 84, 4745–4748.
- [3] Battistelli, G., & Tesi, P. (2015). Detecting topology variations in dynamical networks. In *Proc. IEEE Conf. Decis. Control*, Osaka, Japan (pp. 3349–3354).
- [4] Battistelli, G., & Tesi, P. (2018). Detecting topology variations in networks of linear dynamical systems. *IEEE Trans. Control Netw. Syst.*, 5(3), 1287–1299.
- [5] Buldyrev, S. V., Parshani, R., Paul, G., Stanley, H. E., & Havlin, S. (2010). Catastrophic cascade of failures in interdependent networks. *Nature*, 464(7291), 1025–1028.
- [6] Chen, G. R., Wang, X. F., & Li, X. (2015). *Fundamentals of Complex Networks: Models, Structures and Dynamics*. Wiley.
- [7] Costanzo, J., Materassi, D., & Sinopoli, B. (2017). Using Viterbi and Kalman to detect topological changes in dynamic networks. In *Proc. Amer. Control Conf.*, Seattle, WA, USA (pp. 5410–5415).
- [8] Davoodi, M. R., Khorasani, K., Talebi, H. A., & Momeni, H. R. (2014). Distributed fault detection and isolation filter design for a network of heterogeneous multiagent systems. *IEEE Trans. Control Systems Technology*, 22(3), 1061–1069.
- [9] Dhal, R., Torres, J. A., & Roy, S. (2015). Detecting link failures in complex network processes using remote monitoring. *Physica A*, 437, 36–54.
- [10] Du, H. B., Wen, G. H., Cheng, Y. Y., He, Y. G., & Jia, R. T. (2017). Distributed finite-time cooperative control of multiple high-order nonholonomic mobile robots. *IEEE Trans. Neural Netw. Learn. Syst.*, 28(12), 2998–3006.
- [11] Friedkin, N. E., & Johnsen, E. C. (1999). Influence networks and opinion change. *Adv. Group Processes*, 16(1), 1–29.
- [12] Hao, Y. Q., Duan, Z. S., Chen, G. R., & Wu, F. (2019). Controllability of Kronecker product networks. *Automatica*, 110, 108597.
- [13] Hao, Y. Q., Wang, Q. Y., Duan, Z. S., & Chen, G. R. (2021). Discernibility of Topological Variations for Networked LTI Systems. *IEEE Trans. Autom. Control*, DOI: 10.1109/TAC.2021.3137791.
- [14] Hao, Y. Q., Wang, Q. Y., Duan, Z. S., & Chen, G. R. (2021). The role of reverse edges on consensus performance of chain networks. *IEEE Trans. Syst., Man, Cybern., Syst.*, 51(3), 1757–1765.
- [15] Hegselmann, R., & Krause, U. (2002). Opinion dynamics and bounded confidence: Models, analysis and simulation. *Simulation*, 5(3), 1–24.
- [16] Pandey, P. K., Adhikari, B., & Chakraborty, S. (2020). A diffusion protocol for detection of link failure and utilization of resources in multi-agent systems. *IEEE Trans. Network Science and Engineering*, 7(3), 1493–1507.
- [17] Parlangelì, G., & Valcher, M. E. (2021). On the detection and identification of edge disconnections in a multi-agent consensus network. <https://arxiv.org/abs/2101.06728>.
- [18] Patil, D., Tesi, P., & Trenn, S. (2019). Indiscernible topological variations in DAE networks. *Automatica*, 101, 280–289.
- [19] Rahimian, M. A., Ajorlou, A., & Aghdam, A. G. (2012). Characterization of link failures in multi-agent systems under the agreement protocol. In *Proc. Amer. Control Conf.*, Montreal, QC, Canada (pp. 5258–5263).
- [20] Rahimian, M. A., Ajorlou, A., & Aghdam, A. G. (2012). Detectability of multiple link failures in multi-agent systems under the agreement protocol. In *Proc. IEEE Conf. Decis. Control*, Maui, HI, USA (pp. 118–123).
- [21] Rahimian, M. A., & Preciado, V. M. (2015). Detection and isolation of failures in directed networks of LTI systems. *IEEE Trans. Control Netw. Syst.*, 2(2), 183–192.
- [22] Roman, M. (2005). *Advanced Linear Algebra*. New York: Springer.
- [23] Torres, J. A., Dhal, R., & Roy, S. (2015). Detecting link failures in complex network processes using remote monitoring. In *Proc. 2015 American Control Conference*, Chicago, USA (pp. 189–194).
- [24] Valcher, M. E., & Parlangelì, G. (2019). On the effects of communication failures in a multi-agent consensus network. In *Proc. 23rd International Conference on System Theory, Control and Computing*, Sinaia, Romania (pp. 709–720).

- [25] Wang, L., Chen, G. R., Wang, X. F., & Tang, W. K. S. (2016). Controllability of networked MIMO systems. *Automatica*, 69, 405–409.
- [26] Wen, G. H., Yu, W. W., Hu, G. Q., Cao, J. D., & Yu, X. H. (2015). Pinning synchronization of directed networks with switching topologies: a multiple Lyapunov functions approach. *IEEE Trans. Neural Netw. Learn. Syst.*, 26(12), 3239–3250.
- [27] Wen, G. H., Yu, X. H., Liu, Z. W., & Yu, W. W. (2018). Adaptive consensus-based robust strategy for economic dispatch of smart grids subject to communication uncertainties. *IEEE Trans. Ind. Informat.*, 14(6), 2484–2496.
- [28] Wood, A. D., & Stankovic, J. A. (2002). Denial of service in sensor networks. *Computer*, 35(10), 54–62.
- [29] Zhang, Y., Xia, Y. Q., Zhang, J. H., & Shang, J. (2021). Generic detectability and isolability of topology failures in networked linear systems. *IEEE Trans. Control Netw. Syst.*, 8(1), 500–512.
- [30] Zhao, R., Zuo, Z. Q., & Wang, Y. J. (2022). Event-triggered control for switched systems with denial-of-service attack. *IEEE Trans. Autom. Control*, DOI: 10.1109/TAC.2022.3176442.