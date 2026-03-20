

# A Crypto-Assisted Approach for Publishing Graph Statistics with Node Local Differential Privacy

Shang Liu  
*Kyoto University*  
 Kyoto, Japan  
 shang@db.soc.i.kyoto-u.ac.jp

Yang Cao  
*Hokkaido University*  
 Sapporo, Japan  
 yang@ist.hokudai.ac.jp

Takao Murakami  
*AIST*  
 Tokyo, Japan  
 takao-murakami@aist.go.jp

Masatoshi Yoshikawa  
*Kyoto University*  
 Kyoto, Japan  
 yoshikawa@i.kyoto-u.ac.jp

**Abstract**—Publishing graph statistics under node differential privacy has attracted much attention since it provides a stronger privacy guarantee than edge differential privacy. Existing works related to node differential privacy assume a trusted server who holds the whole graph. However, in many applications, a trusted curator is usually not available due to privacy and security issues. In this paper, for the first time, we investigate the problem of publishing graph statistics under *Node Local Differential privacy (Node-LDP)*, which does not rely on a trusted server. We propose an algorithm to publish the degree distribution with Node-LDP by exploring how to select the graph projection parameter in the local setting and how to execute the graph projection locally. Specifically, we propose a crypto-assisted local projection method based on cryptographic primitives, achieving the higher accuracy than our baseline pureLDP local projection method. Furthermore, we improve our baseline graph projection method from node-level to edge-level that preserves more neighboring information, owning better utility. Finally, extensive experiments on real-world graphs show that crypto-assisted parameter selection owns better utility than pureLDP parameter selection, and edge-level local projection provides higher accuracy than node-level local projection, improving by up to 57.2% and 79.8%, respectively.

**Index Terms**—Degree distribution, Local graph projection, Node local differential privacy, Crypto-assisted

## I. INTRODUCTION

Graph analysis has been receiving more and more attention on social networks, transportation, protein forecast, etc. However, directly publishing graph statistics may leak sensitive information about an individual [1]. Recently, many research works have studied the problem of publishing sensitive graph statistics under differential privacy (DP) [2], [3]. Compared with previous privacy models (e.g.,  $k$ -anonymity,  $l$ -diversity,  $t$ -closeness), differential privacy can resist most private attacks and provide a provable privacy guarantee.

When DP is applied to graph analysis, there are two common variants of DP [4], [5]: Edge Differential Privacy [6]–[9] and Node Differential Privacy [10]–[13]. Intuitively, Edge Differential Privacy guarantees that a query result does not significantly reveal sensitive information about a particular edge in a graph, while Node Differential Privacy protects the information about a node and all its adjacent edges. Obviously, Node Differential Privacy provides a much stronger privacy guarantee than Edge Differential Privacy. Existing works related to Node Differential Privacy are almost in the central (or global) model, where a trusted curator holds the entire graph

data before data publishing. We refer to the above two variants under a central server setting as Edge Central Differential Privacy (Edge-CDP) and Node Central Differential Privacy (Node-CDP), respectively. However, the assumption about a trusted server may not be practical in many applications (i.e., individual contact lists) due to security reasons, such as privacy leaks and breaches in recent years [14]. Local differential privacy (LDP) [15], [16] is a promising model that does not require a trusted server to collect user information. In LDP, each user perturbs its sensitive information by herself and sends perturbed messages to the untrusted server; hence it is difficult for the curator to infer sensitive information with high confidence. We refer to the above two variants of DP without a trusted server as Edge Local Differential Privacy (Edge-LDP) and Node Local Differential Privacy (Node-LDP), respectively.

Although there are many recent studies on publishing statistics under Edge-LDP [17]–[19], to the best of our knowledge, no existing work in literature attempts to investigate graph statistics release under Node-LDP. Basically, it is very challenging to publish graph statistics under Node-LDP due to the lack of global view and prior knowledge about the entire graph. Consider querying the node degree in a social graph, and if two graphs differ in one node, the results may differ at most  $(n-1)$  edges in the worst case, where  $n$  is the number of users. Thus the sensitivity of Node Differential Privacy is  $O(n)$  while that of Edge-DP is  $O(1)$ . Naively scaling the sensitivity of Edge-LDP for achieving Node-LDP suffers the prohibitive utility drop.

Graph projection [10], [11], [13] is the key technique to reduce the high sensitivity, but existing projections are only designed for the central model. When attempting to apply central graph projections into Node-LDP, it is difficult for each local user to project its neighboring information with a limited local view. In central models, with the global view, the server can determine optimal strategies of removing which edges or nodes to maximize the overall utility. However, in the local setting, each user can only see its own information but not other neighboring information. What's more, it is difficult for local users to obtain a graph projection parameter  $\theta$  with high accuracy as they have little knowledge about the entire graph. In general, graph projection transforms a graph into a  $\theta$ -bounded graph whose maximum degree is no more than  $\theta$ . The parameter  $\theta$  plays a vital role as it reduces the sensitivity

![Figure 1: Framework of our methods. The diagram shows a 'Users' group and a 'Server' group. The 'Users' group contains five nodes (v1, v2, v3, v4, v5) connected in a graph. The 'Server' group contains two boxes: 'Aggregate and Compute Overall Utility Loss' and 'Collect and Publish'. The 'Collect and Publish' box contains a bar chart showing the distribution of degrees. The process is: 1. ParameterSelection: A message from the Server to the Users. 2. LocalProjection: A message from the Users to the Server. 3. Send Noisy Degree: A message from the Users to the Server. The 'Collect and Publish' box also shows a bar chart with the following data: degree 0: 1 node, degree 1: 4 nodes, degree 2: 2 nodes, degree 3: 1 node, degree 4: 1 node.](68ac34ff111db52afaa786afcb8346c3_img.jpg)

Figure 1: Framework of our methods. The diagram shows a 'Users' group and a 'Server' group. The 'Users' group contains five nodes (v1, v2, v3, v4, v5) connected in a graph. The 'Server' group contains two boxes: 'Aggregate and Compute Overall Utility Loss' and 'Collect and Publish'. The 'Collect and Publish' box contains a bar chart showing the distribution of degrees. The process is: 1. ParameterSelection: A message from the Server to the Users. 2. LocalProjection: A message from the Users to the Server. 3. Send Noisy Degree: A message from the Users to the Server. The 'Collect and Publish' box also shows a bar chart with the following data: degree 0: 1 node, degree 1: 4 nodes, degree 2: 2 nodes, degree 3: 1 node, degree 4: 1 node.

Fig. 1. Framework of our methods

from  $O(n)$  to  $O(\theta)$ . If  $\theta$  is too small, a large number of edges will be removed during the projection. If  $\theta$  is too large, the sensitivity will become higher and more noise will be added during the protection. Graph projections in the central setting can easily opt for the desirable projection parameter  $\theta$  with some prior knowledge of the whole graph, for instance, the maximum degree, the average degree, etc.; yet it is harder for local users to achieve it, since they have little prior knowledge about the entire graph.

In this paper, we introduce a novel local graph projection method for publishing the degree distribution under Node-LDP by addressing two main challenges: (1) How to obtain the graph projection parameter  $\theta$  in the local setting; (2) How to execute the graph projection locally. The general framework is depicted in detail in Fig. 1, which includes three phases: (1) local users and server collaboratively select a projection parameter  $\theta$  with minimum utility loss (Sec.IV); (2) local users execute local graph projection based on selected parameters (Sec.V); (3) local users perturb individual information and send noisy degrees to the server.

First, to find the optimal projection parameter  $\theta$ , we design a multiple-round protocol to find which parameter has the minimum utility loss. Specifically, for each round, each user calculates the potential utility loss with respect to a certain  $\theta$  and sends to the server for computing the aggregated loss. The utility loss contains sensitive information since it is calculated based on each user's raw data. We design two methods to protect individual messages based on different privacy-enhancing techniques: pureLDP and crypto-assisted. The pureLDP method is a naive local graph projection method under Node-LDP. The obvious disadvantage is that multiple-round adding noise significantly deteriorates the utility. To improve it, we design a crypto-assisted parameter selection method that improves the utility with cryptographic primitives. The key challenge is that aggregated utility loss is computed for the evaluation while individual utility loss can be protected. We first use the order-preserving encryption (OPE) scheme [20], [21] to encode individual information for comparing different utility loss values. Then, we mask the encrypted value with Secure Aggregation (SA) technique [22] to protect the order information of individual utility loss. The masks can be cancelled during the aggregation and the final

aggregated utility loss is protected under OPE scheme.

Second, we propose two different local projection methods based on different granularity, including node-level method and edge-level method. In node-level method, each node is the minimal unit of a graph and correlations among neighboring users will be ignored coarsely. However, this approach loses too much neighboring information that significantly influences the overall utility (detailed analysis in Sec. V-A). Then, we propose an improved approach, edge-level method, where each edge is the minimal unit that is more fine-granularity information. One main challenge is that privacy leakage may happen via communication messages among neighboring users. We represent this message as an operation vector and carefully design a randomized mechanism to perturb each bit of this vector while satisfying Node-LDP. As a result, it is difficult for neighboring users to distinguish whether the current node degree is larger than  $\theta$  or smaller than  $\theta$ .

Our contributions can be summarized as follows:

- We propose and study the problem of publishing the degree distribution under Node-LDP for the first time. We give a detailed description of the problem definition and conclude the research gap. We present an overview of publishing the degree distribution under Node-LDP.
- We design two methods to select the projection parameter  $\theta$  in the local setting: pureLDP and crypto-assisted. Crypto-assisted method guarantees the security of individual utility loss with cryptographic primitives, which achieves a higher accuracy than the baseline pureLDP method.
- We design two local graph projection approaches based on different granularity: node-level and edge-level. The improved edge-level method preserves more information and provides better utility than the baseline node-level method.
- Extensive experiments on real-world graph datasets validate the correctness of our theoretical analysis and the effectiveness of our proposed methods.

## II. PROBLEM DEFINITION AND PRELIMINARIES

### A. Problem Definition

In this paper, we consider an undirected graph with no additional attributes on nodes or edges. An input graph is defined as  $G = (V, E)$ , where  $V = \{v_1, \dots, v_n\}$  is the set of nodes, where  $|V| = n$ , and  $E \subseteq V \times V$  is the set of edges. For each user  $i$ ,  $B_i = \{b_{i1}, b_{i2}, \dots, b_{in}\}$  is its adjacent bit vector, where  $b_{ij} = 1$  if the edge  $(v_i, v_j) \in E$  and  $b_{ij} = 0$  otherwise. The number of adjacent edges for one node  $i$  is the node degree  $d_i$ , namely,  $d_i = \sum_{j=1}^n b_{ij}$ . The server collects a perturbed degree sequence  $seq = \{d_1, d_2, \dots, d_n\}$  from each local user and publishes the degree histogram  $hist(G)$ . The degree distribution  $dist(G)$  can be easily obtained from  $hist(G)$  by counting each degree frequency. Fig. 2 shows an example of degree sequence, degree histogram, and degree distribution, respectively.

We use two common measures to assess the accuracy of our algorithms. First, we use the mean squared error (MSE) [23]

![Figure 2: Example of publishing degree distribution. It consists of three parts: a graph diagram with 4 nodes (1, 2, 3, 4) and edges (1-2, 2-3, 2-4), labeled 'Degree Sequence = (1, 3, 2, 2)'; a 'Degree Histogram' showing the number of nodes for each degree (1: 0, 2: 2, 3: 1, 4: 1); and a 'Degree Distribution' showing the percentage of nodes for each degree (1: 0%, 2: 50%, 3: 25%, 4: 25%).](ad04aa6cb9e7fb36bb7a91e817e2d314_img.jpg)

Figure 2: Example of publishing degree distribution. It consists of three parts: a graph diagram with 4 nodes (1, 2, 3, 4) and edges (1-2, 2-3, 2-4), labeled 'Degree Sequence = (1, 3, 2, 2)'; a 'Degree Histogram' showing the number of nodes for each degree (1: 0, 2: 2, 3: 1, 4: 1); and a 'Degree Distribution' showing the percentage of nodes for each degree (1: 0%, 2: 50%, 3: 25%, 4: 25%).

Fig. 2. Example of publishing degree distribution

to estimate the error between noisy histogram  $hist(G)'$  and original histogram  $hist(G)$ . Generally, the MSE can be computed as  $MSE(hist(G), hist(G)') = \frac{1}{n} \sum_{i=1}^n (hist(G)_i - hist(G)'_i)^2$ , where  $n$  is the number of users in a graph. Also, we compute the mean absolute error (MAE) [24] which can be represented by  $MAE(hist(G), hist(G)') = \frac{1}{n} \sum_{i=1}^n |hist(G)_i - hist(G)'_i|$ .

### B. Preliminaries

Since the trusted third party is impractical, LDP has become the de facto standard of privacy protection to protect individual information. As a graph consists of nodes and edges, there are two definitions when LDP is applied to either of them: edge local differential privacy (Edge-LDP) in Definition 1 and node local differential privacy (Node-LDP) in Definition 2.

**Definition 1 (Edge-LDP):** A random algorithm  $M$  satisfies  $\epsilon$ -Edge-LDP, iff for any  $i \in [n]$ , two adjacent bit vectors  $B_i$  and  $B'_i$  that differ only one bit, and any output  $y \in range(M)$ ,  $Pr[M(B_i) = y] \leq e^\epsilon Pr[M(B'_i) = y]$

**Definition 2 (Node-LDP):** A random algorithm  $M$  satisfies  $\epsilon$ -Node-LDP, iff for any  $i \in [n]$ , two adjacent bit vectors  $B_i$  and  $B'_i$  that differ at most  $n$  bits, and any output  $y \in range(M)$ ,

$$Pr[M(B_i) = y] \leq e^\epsilon Pr[M(B'_i) = y]$$

Node-LDP is clearly a much stronger privacy guarantee than Edge-LDP since it requires hiding the existence of each node along with its incident edges. To our knowledge, however, there are few research works that release graph statistics under Node-LDP. Although Zhang *et al.* [25] consider Node-DP in the local setting where each node represents a software component and an edge represents control flow between components, the directed graphs on the control-flow behavior of different users are mutually independent. We consider a totally different setting where each node represents a user and each edge represents the correlation between neighboring users.

There are two kinds of DP, namely, *bounded* DP and *unbounded* DP [3], [26]. In a bounded DP, two neighboring datasets  $D, D'$  have the same size  $n$  and  $D'$  is obtained from  $D$  by changing or replacing one element. In unbounded DP,  $D'$  can be derived from  $D$  by deleting or adding one element. Here, we use the bounded DP to publish the degree distribution. That is to say, the size of each adjacent bit vector is equal to  $n$ , where  $n$  is the number of users. Node-LDP satisfies the post-processing property (Theorem 1) and the composition property (Theorem 2) [2].

**Theorem 1 (Post-Processing):** If a randomized algorithm  $R$  satisfies  $\epsilon$ -DP, then for an arbitrary randomized algorithm  $S$ ,  $S \circ R$  also satisfies  $\epsilon$ -DP.

**Theorem 2 (Composition Property):**  $\forall \epsilon \geq 0, k \in \mathbb{N}$ , the family of  $\epsilon$ -DP mechanism satisfies  $t\epsilon$ -DP under  $t$ -fold adaptive composition.

To satisfy DP, one way to add some noise into the query result. In the Laplace mechanism (Theorem 3) [2], [3], given the privacy budget  $\epsilon$  and sensitivity  $\Delta$ , one publishes the result after adding  $Lap(\frac{\Delta}{\epsilon})$  noise.

**Theorem 3 (Laplace Mechanism):** For any function  $f$ , the Laplace mechanism  $A(D) = f(D) + Lap(\frac{\Delta f}{\epsilon})$  satisfies  $\epsilon$ -DP.

## III. OVERVIEW OF PROPOSED METHODS

We aim to design a method for publishing the degree distribution that approximates the original distribution as possible while satisfying the strict Node-LDP. Our proposed methods support the following functions: 1) obtaining the graph projection parameter  $\theta$  in the local setting; 2) conducting the graph projection locally; 3) publishing the degree distribution under Node-LDP.

#### Algorithm 1 Publishing the degree distribution

---

**Input:** Adjacent bit vectors  $\{B_1, \dots, B_n\}$ ,  
privacy budget  $\epsilon_1, \epsilon_2, \epsilon_3$

**Output:** A noisy degree distribution  $dist(G)'$

- 1:  $\theta \leftarrow \text{SelectParameter}(\{B_1, \dots, B_n\}, \epsilon_1)$  // Sec. IV  
/\* User side. \*/
- 2: **for** each user  $i \in \{1, 2, \dots, n\}$  **do**
- 3:  $\hat{d}_i \leftarrow \text{LocalProjection}(B_i, \theta, \epsilon_2)$  // Sec. V
- 4:  $d'_i \leftarrow \hat{d}_i + \text{Lap}(\frac{2\theta}{\epsilon_3})$
- 5: User  $i$  sends  $d'_i$  to server
- 6: **end for** /\* Curator side. \*/
- 7: Curator collects all noisy degree  $d'_i$
- 8: **return**  $dist(G)'$

---

We provide an overview of our solutions in Algorithm 1. First, a private parameter selection method is designed to select the projection parameter with minimum utility loss in the local setting (Section IV). The curator collects individual utility loss from local users and evaluates each candidate projection parameter  $k$  by computing the aggregated utility loss. To protect sensitive individual utility loss during communications, we first propose one naive approach, pureLDP parameter selection, which adds noise into individual utility loss. However, this method adds too much noise to destroy the order information of different aggregated utility loss, significantly influencing the selection accuracy. Then, we propose an improved crypto-assisted parameter selection method using cryptographic primitives. Specifically, the individual utility loss is encrypted by order-preserving encryption (OPE) [27] scheme where the numerical order in the plaintext domain will be preserved in the ciphertext domain. To prevent leaking the order information of individual utility loss while preserving the order of the aggregated utility loss, we add one mask into encrypted values with Secure Aggregation technique [22]. The added masks are cancelled after the aggregation and the final aggregated utility loss is protected under OPE scheme.

Second, as soon as the projection parameter is decided, each user can execute the local projection (Section V). Compared

with the Node-CDP, it is more difficult for each user to execute the local projection due to the limited local view of the entire graph. We first give a baseline node-level approach that is motivated by graph projection [19] with Edge-LDP. In node-level local projection, the node is the minimal unit and correlations among users are ignored. It is easy to deploy but lose much information that significantly influences the utility. Then we design an improved edge-level local projection where each edge is the minimal unit during the projection. The key challenge is that information leakage may happen via mutual edges among neighboring users. For example, neighboring users may know that the current degree is larger than or less than  $\theta$  during the local projection. We represent this sensitive message as each bit in an operation vector and design a randomized mechanism to perturb each bit. Thus neighboring users cannot distinguish the current node degree whether larger than  $\theta$  or smaller than  $\theta$ .

Third, after finishing the local projection, each user perturbs its projected degree using the Laplace mechanism. Here, the sensitivity is  $2\theta$  since any change of one edge will make an effect on two node degrees. Then, they send the noisy degree to the server. The curator collects the degree sequence and publishes the degree histogram and degree distribution.

## IV. PROJECTION PARAMETER SELECTION

### A. PureLDP Selection

Intuitively, the server can help local users select the parameter with the minimum utility loss from the candidate set  $\{1, 2, \dots, K\}$  through multiple-round communications. We design a utility loss function to evaluate each candidate parameter  $k$ . Our utility loss function has two parts, as shown in Equation 1, which includes projection utility loss during the local projection and publishing utility loss from added Laplace noise. The publishing utility loss  $E_D$  is usually a constant value. For example, the publishing utility loss of degree distribution is equal to the variance, namely,  $E_D = n.2(\frac{2k}{\epsilon_3})^2 = \frac{8nk^2}{\epsilon_3^2}$ . The projection utility loss  $E_P$  is aggregated by all individual projection utility losses, i.e.,  $E_P = \sum_{i=1}^n \{d_i - k | v_i \in V, d_i > k\}$ . But directly collecting each individual utility loss from local users may reveal personal information. In baseline method, we use the Laplace mechanism to provide the privacy guarantee and its sensitivity is  $(n - 1 - k)$  in Node-LDP, as shown in Lemma 1.

$$F(k) = E_P + E_D, \quad (1)$$

$$E_P = \sum_{i=1}^n \{d_i - k | v_i \in V, d_i > k\}$$

$$E_D = n.2(\frac{2k}{\epsilon_3})^2 = \frac{8nk^2}{\epsilon_3^2}$$

**Lemma 1:** For any projection loss  $|d_i - \hat{d}_i|$  and  $|d_i - \hat{d}_i|'$ , we have

$$||d_i - \hat{d}_i| - |d_i - \hat{d}_i|| \leq (n - 1 - k)$$

**Proof of Lemma 1:** Given the graph projection parameter is  $k$ , for each node degree  $d_i$ , if  $d_i \leq k$ , projected node degree

#### Algorithm 2 PureLDP parameter selection

---

**Input:** Adjacent bit vectors  $\{B_1, \dots, B_n\}$ , privacy budget  $\epsilon_1$   
**Output:** Projection parameter  $\theta$

```

1: for each integer  $k \in \{1, 2, \dots, K\}$  do
2:   /* User side. */
3:   for each user  $i \in \{1, 2, \dots, n\}$  do
4:      $\hat{d}_i \leftarrow \text{LocalProjection}(B_i, k)$  // Sec. V
5:      $d_i \leftarrow \sum_{j=1}^n b_{i,j}$ 
6:      $E_{P_{k,i}} \leftarrow |d_i - \hat{d}_i| + \text{Lap}(\frac{n-1-k}{\epsilon_1/K})$ 
7:     User  $i$  sends  $E_{P_{k,i}}$  to server
8:   end for
9:   /* Curator side. */
10:   $E_{P_k} \leftarrow \sum_{i=1}^n E_{P_{k,i}}$ 
11:   $\theta \leftarrow k$  when  $(E_{P_k} + E_D)$  is minimum
12: return  $\theta$ 

```

---

$\hat{d}_i$  will remain the original value, namely,  $\hat{d}_i = d_i$ ; otherwise,  $\hat{d}_i = k$ . Thus, we have

$$|d_i - \hat{d}_i| = \begin{cases} d_i - \theta, & d_i > k \\ 0, & d_i \leq k \end{cases}$$

Since the maximum node degree is  $(n - 1)$ , the projection loss value is bounded by  $(n - 1 - k)$ .

**Algorithm.** Algorithm 2 presents the formal description of pureLDP parameter selection. It takes as input a graph  $G$  that is represented as bit vectors  $\{B_1, \dots, B_n\}$ , the privacy budget  $\epsilon_1$ , and the size of candidate parameter  $K$ . For each candidate parameter  $k$ , the original graph is first projected to  $k$ -bounded graph using the local graph projection method (in Section V). Then, each user computes the projection utility loss and adds the Laplace noise into individual utility loss with the sensitivity  $(n - 1 - k)$ . After collecting all noisy individual projection utility loss, the server computes the sum of aggregated projection utility loss and publishing utility loss. Finally, the parameter  $\theta$  is selected when the overall utility loss is the minimum and server sends this  $\theta$  to each local user.

**Limitation.** Much noise is added into the true individual utility loss, which significantly destroys the order information of aggregated utility loss. To capture the impact of adding Laplace noise on the accuracy of pureLDP parameter selection method, we execute experiments on Wikipedia vote network from SNAP [28]. As shown in Fig. 3, the left figure presents the impact of added noise on the order of individual utility loss when  $\theta = 20$ , and the difference between true and noisy utility loss is up to 85%. The right one shows the influence

![Figure 3: The impact of added noise on order information. (a) Under theta=20: A line graph showing 'Message' (y-axis, 0 to 5) vs 'User' (x-axis). The 'true' line (solid black) is relatively flat, while the 'noisy' line (dashed red) shows significant fluctuations. (b) Under various theta: A line graph showing 'Aggregation' (y-axis, 1 to 3) vs 'theta' (x-axis, 5 to 20). The 'true' line (solid black) decreases linearly, while the 'noisy' line (dashed red) fluctuates around it.](2292e808e116eef6d599b629d5fcb01f_img.jpg)

Figure 3: The impact of added noise on order information. (a) Under theta=20: A line graph showing 'Message' (y-axis, 0 to 5) vs 'User' (x-axis). The 'true' line (solid black) is relatively flat, while the 'noisy' line (dashed red) shows significant fluctuations. (b) Under various theta: A line graph showing 'Aggregation' (y-axis, 1 to 3) vs 'theta' (x-axis, 5 to 20). The 'true' line (solid black) decreases linearly, while the 'noisy' line (dashed red) fluctuates around it.

Fig. 3. The impact of added noise on order information

on the order of aggregated utility loss under various  $\theta$  and the difference is up to 90%. Finally, the accuracy of selecting projection parameter  $\theta$  is influenced significantly.

### B. Crypto-assisted Selection

Our goal is that each individual projection utility loss can be protected when the order of aggregated utility loss is preserved. Order-preserving encryption (OPE) scheme [29], [30] can achieve this idea that the  $i$ -th data in the plaintext domain is transformed to the  $i$ -th data in the ciphertext domain, so the numerical order among plaintexts is preserved among ciphertexts. Thus when individual utility loss are encrypted by OPE scheme, the numerical order of individual utility loss can be preserved and the order of aggregated utility loss is also preserved. But there is the other problem that the order of aggregated utility loss is preserved while the order of individual utility loss is revealed to server. Next, we use the secure aggregation [22] to mask encoded individual utility loss, and these masks can be cancelled during the aggregation.

**OPE Schemes.** There are many existing works related to OPE scheme. For example, Popa *et al.* [27] proposed an interactive OPE scheme between the client and the server, which allows the encrypted state to update over time as the new values are inserted. The server organizes the encrypted values by maintaining a binary search tree, namely, OPE tree. To reduce the high cost of the encryption, Kerschbaum *et al.* [31] designed a more efficient OPE scheme that uses a dictionary to keep the state and thus does not need to store too much data. Roche *et al.* [32] proposed an alternative approach to optimize the heavy insertion of OPE schemes. It is very efficient at insertion and has a lower communication cost, but it provides only a partial order. Here, we choose a linear OPE scheme [33] to encode individual utility loss since it can be directly extended for the local setting.

**Secure Aggregation** [22]. Consider a curator with  $n$  users where user  $i \in [n]$  has its private local vector  $x_i$ . The objective of server is to compute the sum of models  $\sum_{i \in n} x_i$  without getting any other information on private local data. Suppose each pair of users  $(i, j), i < j$  agree on some random vector  $s_{i,j}$ . If user  $i$  adds  $s_{i,j}$  to  $x_i$  and  $j$  subtracts it from  $x_j$ , then the mask  $s_{i,j}$  will be canceled when their vectors are added, but their true inputs will be concealed without revealing. Formally, each masked value can be computed:

$$y_i = x_i + \sum_{j \in n: i < j} s_{i,j} - \sum_{j \in n: i > j} s_{i,j} \pmod{R}$$

Then server collects  $y_i$  and computes:

$$\begin{aligned} z &= \sum_{i \in n} y_i \\ &= \sum_{i \in n} \left( x_i + \sum_{j \in n: i < j} s_{i,j} - \sum_{j \in n: i > j} s_{i,j} \right) \\ &= \sum_{i \in n} x_i \pmod{R} \end{aligned}$$

Based on above two cryptographic primitives, we propose a crypto-assisted parameter selection method, as presented in Al-

#### --- Algorithm 3 Crypto-assisted parameter selection ---

**Input:** Adjacent bit vectors  $\{B_1, \dots, B_n\}$ , security parameters  $a, b$   
**Output:** Projection parameter  $\theta$

- 1: **for** each integer  $k \in \{1, 2, \dots, K\}$  **do**
- 2:   /\* User side. \*/
- 3:   **for** each user  $i \in \{1, 2, \dots, n\}$  **do**
- 4:      $\hat{d}_i \leftarrow \text{LocalProjection}(B_i, k)$  // Sec. V
- 5:      $d_i \leftarrow \sum_{j=1}^n b_{i,j}$
- 6:      $\text{noise} \leftarrow \text{randint}(0, a - 1)$
- 7:      $r \leftarrow \text{PRG}(\text{seed})$
- 8:      $\text{mask} = \sum_{j=i+1}^{n-1} r_{i,j} - \sum_{j=1}^{i-1} r_{i,j}$
- 9:      $\text{Enc}_{T_{k,i}} \leftarrow a * |d_i - \hat{d}_i| + b + \text{noise} + \text{mask}$
- 10:    User  $i$  sends  $\text{Enc}_{T_{k,i}}$  to server
- 11:   **end for**
- 12:   /\* Curator side. \*/
- 13:    $\text{Enc}_{T_k} \leftarrow \sum_{i=1}^n \text{Enc}_{T_{k,i}}$
- 14:    $\theta \leftarrow k$  when  $(\text{Enc}_{T_k} + E_D)$  is minimum
- 15: **end for**
- 16: **return**  $\theta$

---

gorithm 3. First, we use the linear OPE scheme [33] to encode individual utility loss, namely,  $f(x) = a * |d_i - \hat{d}_i| + b + \text{noise}$ . Here security parameters  $a$  and  $b$  are kept secret from the server and the noise is randomly selected from  $[0, a - 1]$ . Second, to hide the order of individual utility loss, we add one mask into the encoded values of the OPE scheme using SA. For each user  $i$ , it and the rest other  $n - 1$  users agree on common seeds. Then local users generate the random numbers  $r$  with the common seeds by the pseudorandom generator (PRG) [34] and add into the individual utility loss. Finally, the server collects all encrypted individual projection utility loss and computes the aggregated utility loss. The added masks can be cancelled with each other after aggregation and any information about individuals cannot be leaked. The final aggregated utility loss is still protected under OPE scheme.

## V. LOCAL PROJECTION METHODS

### A. Node-level Local Projection

Local scenarios make projection operations challenging, since no party owns the entire graph and local users cannot easily add or remove any edges. We propose a node-level projection method where each node is the minimal unit. As presented in Algorithm 4, it inputs an adjacent bit vector and projection parameter  $\theta$ . Each local user first counts the number of neighboring edges. If node degree  $d_i$  is larger than  $\theta$ , projected degree  $\hat{d}_i$  will be directly set as  $\theta$ ; otherwise,  $\hat{d}_i$  remains the original value.

**Limitations.** Although node-level projection is easy to implement, it omits correlations among neighboring users coarsely, influencing the accuracy significantly. For example, we have a simple graph with five nodes and some edges, as shown in Fig. 4. The original histogram can be represented as  $H_1 = (0, 3, 1, 1, 0)$ . Assume that the projection parameter  $\theta = 1$ , the projected degree sequence becomes  $Seq_1 = (1, 1, 1, 1, 1)$  and the current histogram is  $H_2 = (0, 5, 0, 0, 0)$

#### --- **Algorithm 4** Node-level Local Projection ---

**Input:** Adjacent bit vector  $B_i=\{b_{i1}, \dots, b_{in}\}$ , projection parameter  $\theta$

**Output:**  $\theta$ -bounded node degree  $\hat{d}_i$

```

1:  $d_i \leftarrow \sum_{j=1}^n b_{i,j}$ 
2: if  $d_i > \theta$  then
3:    $\hat{d}_i = \theta$ 
4: else
5:    $\hat{d}_i \leftarrow d_i$ 
6: end if
7: return  $\hat{d}_i$ 

```

---

![Figure 4: Example of degree histogram. The left part shows a graph with nodes A, B, C, D, and E. Node A is connected to B (degree 1), B is connected to C (degree 2), C is connected to D (degree 3) and E (degree 4). The degree sequence is (1, 2, 3, 1, 1). The right part shows a degree histogram with the x-axis labeled 'Degree' (0 to 4) and the y-axis labeled '# of nodes' (0 to 3). The histogram shows: Degree 1: 4 nodes (yellow bar), Degree 2: 1 node (blue bar), Degree 3: 1 node (blue bar), Degree 4: 1 node (blue bar).](ac99eff233b8fe51d30f499e7413c345_img.jpg)

Figure 4: Example of degree histogram. The left part shows a graph with nodes A, B, C, D, and E. Node A is connected to B (degree 1), B is connected to C (degree 2), C is connected to D (degree 3) and E (degree 4). The degree sequence is (1, 2, 3, 1, 1). The right part shows a degree histogram with the x-axis labeled 'Degree' (0 to 4) and the y-axis labeled '# of nodes' (0 to 3). The histogram shows: Degree 1: 4 nodes (yellow bar), Degree 2: 1 node (blue bar), Degree 3: 1 node (blue bar), Degree 4: 1 node (blue bar).

Fig. 4. Example of degree histogram

after node-level projection. We can compute the projection loss:  $MSE(H_1, H_2)=\frac{6}{5}$ . If correlations are considered, any change in mutual edges will update two neighboring adjacent bit vectors. For example, if edge 2 and 3 are removed to bound all degrees, the degree sequence will become  $Seq_2=(1, 1, 1, 0, 1)$  and the degree histogram will be  $H_3=(1, 4, 0, 0, 0)$ . The projection loss can be computed:  $MSE(H_1, H_3)=\frac{4}{5}$ . Obviously, node-level method loses more edge information, which significantly affects overall utility. What's more, the characteristic of degree distribution is destroyed by node-level projection. For instance, it is not easy to find a real-world graph that is represented by the sequence  $Seq_1$ .

Generally, we assume that the number of users in a graph is  $n$ , projection parameter is  $\theta$ , and original degree histogram is  $H_1=(h_1, h_2, \dots, h_n)$ . If there are  $m$  nodes with degree larger than  $\theta$ , we can get the projected histogram  $H_2=(h_1, h_2, \dots, h_\theta + m, 0, \dots, 0)$  using node-level projection. On the other hand, if mutual edge information is considered during the projection, the new histogram will become  $H_3=(h_1 + t_1, h_2 + t_2, \dots, h_\theta + t_m, 0, \dots, 0)$ , where  $t_i \in \mathbb{Z}$  ( $i \in [1, m]$ ) is the variation of each bin in the histogram. We refer to this method as edge-level projection method. One mutual edge connects two nodes and there are two cases during the edge-level local projection: (1) two node degrees are both over  $\theta$ . The final histogram of edge-level is same with that of node-level. (2) one node degree is larger than  $\theta$  and the other one is smaller than  $\theta$ . The change from the former one is same with the first case. The influence from the latter can be cancelled finally. Thus, we can easily achieve  $m = t_1 + t_2 + \dots + t_m$ . Then we can compute their projection loss, namely,  $MSE(H_1, H_2)=\frac{m^2}{n} = \frac{1}{n}(t_1 + t_2 + \dots + t_m)^2$  and  $MSE(H_1, H_3)=\frac{1}{n}(t_1^2 + t_2^2 + \dots + t_m^2)$ . Since  $(t_1 + t_2 + \dots + t_m)^2 \geq (t_1^2 + t_2^2 + \dots + t_m^2)$ , we can get  $MSE(H_1, H_2) \geq MSE(H_1, H_3)$ . Therefore, the result of node-level projection method is not desirable.

#### --- **Algorithm 5** Edge-level Local Projection ---

**Input:** Adjacent bit vector  $B_i=\{b_{i1}, \dots, b_{in}\}$ , projection parameter  $\theta$ , privacy budget  $\varepsilon_2$

**Output:**  $\theta$ -bounded node degree  $\hat{d}_i$

```

1:  $R_i=[0] \times \hat{d}_i$  // Record which edges will be deleted
2:  $d_i \leftarrow \sum_{j=1}^n b_{i,j}$ 
3: if  $d_i \geq \theta$  then
4:   Randomly select  $(d_i - \theta)$  bits from  $R_i$  and set '1'
5:   for each  $r_{ij} \in R_i$  do
6:

```

$$r'_{ij} = \begin{cases} r_{ij} & w.p. \frac{\theta}{d_i} \\ 1 - r_{ij} & w.p. \frac{d_i - \theta}{d_i} \end{cases}$$

```

7:   end for

```

```

8: else

```

```

9:   for each  $r_{ij} \in R_i$  do

```

```

10:    if  $\frac{d_i - \theta}{d_i} \leq \frac{e^{\varepsilon_2} - 1}{e^{\varepsilon_2} - e^{-\varepsilon_2}}$  then

```

```

11:

```

$$r'_{ij} = \begin{cases} r_{ij} & w.p. 1 - \frac{e^{-\varepsilon_2}(d_i - \theta)}{d_i} \\ 1 - r_{ij} & w.p. \frac{e^{-\varepsilon_2}(d_i - \theta)}{d_i} \end{cases}$$

```

12:    else

```

```

13:

```

$$r'_{ij} = \begin{cases} r_{ij} & w.p. \frac{e^{\varepsilon_2}\theta}{d_i} \\ 1 - r_{ij} & w.p. \frac{d_i - e^{\varepsilon_2}\theta}{d_i} \end{cases}$$

```

14:    end if

```

```

15:   end for

```

```

16: end if

```

```

17: for each  $r_{ij} \in R_i$  do

```

```

18:   if  $r_{ij} = 1$  then

```

```

19:      $b_{ij} = 0$  and  $b_{ji} = 0$ 

```

```

20:   end if

```

```

21: end for

```

```

22: return  $\hat{d}_i$ 

```

---

### B. Edge-level Local Projection

Based on above discussions, if we consider the correlation among users, more edge information will be reserved after the projection. However, unlike Node-CDP where the trusted server can decide the optimal strategies of removing which edges or nodes to maximize the overall utility, it is difficult for a local user to update the mutual edges. The key challenge is that any change in the edges may leak individual sensitive information via mutual edges. For example, if one node degree  $d_i$  is larger than  $\theta$ , it will delete some edges. At the same time, this user  $i$  will send messages to its neighboring users to update their adjacent bit vectors. The message itself reveals that the current node degree may be larger than  $\theta$ . We design an edge-level method to protect this sensitive message.

**Security Assumptions.** We assume that 1) the communication between neighboring users is perfectly anonymous, that's to say, the third party (e.g., server or third user) cannot know the communication exists or not; 2) the user does not reveal sensitive neighboring information to other users, for example, B will not tell C that A is one of its friends or not. Based on above assumptions, one edge is only visible to two neighboring

TABLE I  
RANDOMIZED PROJECTION VECTOR

| Pr            | 0     | 1   |
|---------------|-------|-----|
| $< \theta$    | $1-x$ | $x$ |
| $\geq \theta$ | $1-p$ | $p$ |

users and other edges are in a data-invisible way. Thus, the communication message is just one bit and the sensitivity becomes  $O(1)$ .

**Algorithm.** We propose the edge-level projection method to improve node-level method and the edge is the minimal unit during the projection, as shown in Algorithm 5. Privacy leakage may occur when the local projection is performed since the sensitive messages are sent to neighboring users. We represent this message as an operation vector  $R_i = \{r_{i1}, \dots, r_{id_i}\}$ , and the size of  $R_i$  is  $d_i$ . If  $r_{ij} = 1$ , the corresponding edges in two neighbor lists will be removed; otherwise, they remain the same. We carefully perturb each bit of the operation vector to make two cases indistinguishable: node degree  $d_i$  is larger than  $\theta$  or  $d_i$  is smaller than  $\theta$ . Ideally, we want to flip each bit of the projection bit vector with probability in Table I, where  $p = \frac{d_i - \theta}{d_i}$  and  $x = 0$ . Obviously, when  $x = 0$ , our randomized mechanism cannot satisfy the Node-LDP. To satisfy the Node-LDP, we have the following inequation:

$$\begin{cases} e^{-\varepsilon_2} \leq \frac{x}{p} \leq e^{\varepsilon_2} \\ e^{-\varepsilon_2} \leq \frac{1-x}{1-p} \leq e^{\varepsilon_2} \end{cases}$$

Then, we can bound the scope of  $x$  as follows:

$$\begin{cases} pe^{-\varepsilon_2} \leq x \leq pe^{\varepsilon_2} \\ (p-1)e^{\varepsilon_2} + 1 \leq x \leq (p-1)e^{-\varepsilon_2} + 1 \end{cases}$$

When  $d_i < \theta$ , we want to preserve more edges during projection, that is to say, the number of '1' in projection bit vector is as small as possible. Thus we have

$$x = \begin{cases} pe^{-\varepsilon_2}, & pe^{-\varepsilon_2} \geq (p-1)e^{\varepsilon_2} + 1 \\ (p-1)e^{\varepsilon_2} + 1, & pe^{-\varepsilon_2} < (p-1)e^{\varepsilon_2} + 1 \end{cases}$$

After randomizing the bits of the projection bit vector, each user updates the adjacent bit vector according to randomized bit vector (Line 19). Then, local users count the number of edges and obtain the bounded degree  $\hat{d}_i$ .

## VI. ANALYSIS AND DISCUSSIONS

**Privacy Budget Allocation.** As shown in Algorithm 1, there are three kinds of privacy budgets. Our goal is to find the optimal privacy allocation scheme with the best utility. Without loss of generality, we assume that the overall privacy budget is  $\varepsilon$ ,  $\varepsilon_3 = \alpha\varepsilon$ , and  $\varepsilon_1 + \varepsilon_2 = (1 - \alpha)\varepsilon$ . For inner privacy budget allocation of local graph projection, we distribute the same privacy budget for the projection parameter selection and executing the local graph projection, namely,  $\varepsilon_1 = \varepsilon_2$ . We find the optimal  $\alpha$  with the least utility loss by conducting many experiments for different cases, as shown in Table II. And we use the optimal  $\alpha$  for each case in the next experiments.

**Selection of Parameter  $K$ .** In Algorithm 2 and Algorithm 3, the parameter  $K$ , namely, the size of the candidate pool, plays a significant role in the tradeoff between utility and privacy. When the size  $K$  is larger, more noise will be

TABLE II  
OPTIMAL PRIVACY ALLOCATION SCHEME  $\alpha$

| $\varepsilon$ | Ca-HepPh | Cit-HepPh | Twitter | Com-DBLP |
|---------------|----------|-----------|---------|----------|
| 0.5           | 0.895    | 0.927     | 0.945   | 0.945    |
| 1             | 0.944    | 0.937     | 0.949   | 0.947    |
| 1.5           | 0.901    | 0.940     | 0.944   | 0.948    |
| 2             | 0.948    | 0.946     | 0.947   | 0.937    |
| 2.5           | 0.944    | 0.922     | 0.948   | 0.943    |
| 3             | 0.944    | 0.948     | 0.941   | 0.940    |

TABLE III  
OPTIMAL PARAMETER  $\theta$

| $\varepsilon$ | Ca-HepPh | Cit-HepPh | Twitter | Com-DBLP |
|---------------|----------|-----------|---------|----------|
| 0.5           | 3        | 4         | 18      | 13       |
| 1             | 9        | 7         | 31      | 17       |
| 1.5           | 15       | 10        | 41      | 20       |
| 2             | 19       | 12        | 43      | 23       |
| 2.5           | 24       | 15        | 45      | 25       |
| 3             | 26       | 18        | 48      | 27       |

added by the pureLDP parameter selection and time overhead becomes higher. Similarly, the running time of crypto-assisted selection method will be higher. But if the  $K$  becomes smaller, the optimal projection parameter  $\alpha$  is not covered possibly. We conduct extensive experiments and find the optimal parameter  $\alpha$  for each case, as shown in Table III. In our paper, we use  $K = 50$  that is ample to cover the optimal parameter  $\alpha$  of different cases.

**Time Complexity.** As shown in Table IV, we conclude the running time complexity of different combinations theoretically,  $|V|$  and  $|E|$  represent the number of nodes and edges respectively. Node-level local projection method transforms each node degree into  $\theta$ -bounded degree directly, which takes time  $O(|V|)$ . In contrast, edge-level local projection method needs to traverse each edge for each node, resulting an  $O(|V| \cdot |E|)$  running time. PureLDP parameter selection method selects the optimal parameter  $\theta$  from  $K$  candidates and for each candidate  $k$ , each user has to compute the projection loss, which takes time at most  $O(K \cdot |V|)$ . By comparison, for each candidate parameter  $k$  of crypto-assisted selection method, each user has to communicate with the other  $(|V| - 1)$  users to determine the seed, resulting an  $O(K \cdot |V|^2)$  running time overhead.

TABLE IV  
RUNNING TIME COMPLEXITY

|            | pureLDP                          | crypto-assisted                    |
|------------|----------------------------------|------------------------------------|
| Node-level | $O( V  + K \cdot  V )$           | $O( V  + K \cdot  V ^2)$           |
| Edge-level | $O( V  \cdot  E  + K \cdot  V )$ | $O( V  \cdot  E  + K \cdot  V ^2)$ |

**Security Analysis.** Publishing the degree distribution in Algorithm 1 is under the following privacy guarantee.

**Lemma 2:** Publishing the degree distribution satisfies  $(\varepsilon_1/K + \varepsilon_2 + \varepsilon_3)$ -Node-LDP.

**Proof of Lemma 2:** In Algorithm 1, SelectParameter(.) (Line 1) uses the Laplace with privacy budget  $\varepsilon_1/K$ ,  $K$  is the number of candidate parameters. Executing the local projection (Line 3) uses our proposed mechanism and satisfies Node-LDP

![Figure 5: The MSE and MAE of algorithms on different graphs. The figure consists of eight subplots arranged in a 2x4 grid. The top row (a-d) shows MSE on a logarithmic y-axis (10^3 to 10^4) for four datasets: (a) Ca-HepPh, (b) Cit-HepPh, (c) Twitter, and (d) Com-DBLP. The bottom row (e-i) shows MAE on a linear y-axis for the same datasets: (e) Ca-HepPh, (f) Cit-HepPh, (g) Twitter, (h) Twitter, and (i) Com-DBLP. Each subplot compares four methods: PureLDP + NodeProj (blue circles), CryptoAssisted + NodeProj (orange stars), PureLDP + EdgeProj (green triangles), and CryptoAssisted + EdgeProj (red diamonds). In all cases, the MSE and MAE decrease as the privacy budget ε increases from 0.5 to 3.0. The CryptoAssisted + EdgeProj method consistently achieves the lowest MSE and MAE across all datasets and privacy budgets.](c3c305cefbac2e7b13be34ab87054d1e_img.jpg)

Figure 5: The MSE and MAE of algorithms on different graphs. The figure consists of eight subplots arranged in a 2x4 grid. The top row (a-d) shows MSE on a logarithmic y-axis (10^3 to 10^4) for four datasets: (a) Ca-HepPh, (b) Cit-HepPh, (c) Twitter, and (d) Com-DBLP. The bottom row (e-i) shows MAE on a linear y-axis for the same datasets: (e) Ca-HepPh, (f) Cit-HepPh, (g) Twitter, (h) Twitter, and (i) Com-DBLP. Each subplot compares four methods: PureLDP + NodeProj (blue circles), CryptoAssisted + NodeProj (orange stars), PureLDP + EdgeProj (green triangles), and CryptoAssisted + EdgeProj (red diamonds). In all cases, the MSE and MAE decrease as the privacy budget ε increases from 0.5 to 3.0. The CryptoAssisted + EdgeProj method consistently achieves the lowest MSE and MAE across all datasets and privacy budgets.

Fig. 5. The MSE and MAE of algorithms on different graphs

for  $\varepsilon_2$ . And publishing the distribution with Laplace Mechanism using  $\varepsilon_3$ . According to the post-processing theorem and composition property, Algorithm 1 satisfies  $(\varepsilon_1/K + \varepsilon_2 + \varepsilon_3)$ -Node-LDP.

## VII. EXPERIMENTAL EVALUATION

In this section, we would like to answer the following questions:

- What is the tradeoff between utility and privacy of our proposed methods?
- What are results of different privacy budget allocation schemes?
- How much time do our proposed algorithms take?

### A. Datasets and Setting

Our experiments run in python on a server with Intel Core i9-10920X CPU, 256GB RAM running Ubuntu 18.04 LTS. We use four real-world graph datasets from SNAP [28], which are also used in [10], [23]. And we preprocess all graph datasets to be undirected and symmetric graphs. Table V presents more details about every graph dataset, including the number of nodes  $|V|$ , the number of edges  $|E|$ , and the number of edges after preprocessing  $|E'|$  after preprocessing. In all experiments, we vary the privacy budget  $\varepsilon$  from 0.5 to 3. By default, we set hyper-parameter  $K=50$  as we discussed above. All of our experimental results are the average values

TABLE V  
DETAILS OF GRAPH DATASETS

| Graph     | $ V $   | $ E $     | $ E' $    |
|-----------|---------|-----------|-----------|
| Ca-HepPh  | 12,008  | 118,521   | 474,020   |
| Cit-HepPh | 34,546  | 421,578   | 843,156   |
| Twitter   | 81,306  | 1,768,149 | 3,536,298 |
| Com-DBLP  | 317,080 | 1,049,866 | 2,099,732 |

computed from 20 runs. We use ‘PureLDP’, ‘CryptoAssisted’, ‘NodeProj’ and ‘EdgeProj’ to represent pureLDP parameter selection, crypto-assisted parameter selection, node-level local graph projection and edge-level local graph projection respectively. Thus we have four different combinations to publish the degree distribution.

### B. Relation between $\varepsilon$ and MSE, MAE

As shown in Fig. 5, the utility of each combination method becomes better as the privacy budget  $\varepsilon$  increases. We can find that ‘CryptoAssisted+EdgeProj’ method always performs the best in most cases, while the results of ‘PureLDP+NodeProj’ method are always the worst. To be specific, the MSE of ‘CryptoAssisted+EdgeProj’ method is less than that of ‘PureLDP+NodeProj’ by up to 87.2% on Twitter when  $\varepsilon = 2.5$ . The MAE of ‘CryptoAssisted+NodeProj’ method is larger than that of ‘CryptoAssisted+EdgeProj’ method by up to 66.4% in Twitter when  $\varepsilon = 3$ . The reason that ‘CryptoAssisted+EdgeProj’ method sometimes performs not the best in terms of MAE when  $\varepsilon = 0.5$  is because our utility loss function uses the MSE as the evaluation metric, which makes a little influence on results of MAE, particularly when  $\varepsilon$  is very small. The results of pureLDP parameter projection are always worse than that of crypto-assisted parameter projection since the latter protects individual utility loss while preserving the order information of the aggregated utility loss accurately. Also, due to more information is preserved, edge-level local projection method performs much better than node-level local projection method. Overall, our proposed ‘CryptoAssisted+EdgeProj’ method improves our baseline ‘PureLDP+NodeProj’ approach for publishing the degree distribution under Node-LDP.

![Figure 6: The MSE on different graphs, varying alpha. The figure contains four bar charts (a) Ca-HepPh, (b) Cit-HepPh, (c) Twitter, and (d) Com-DBLP. Each chart plots MSE (y-axis) against epsilon (x-axis) for four alpha values: 0.3, 0.6, 0.9, and best alpha. In all cases, the 'best alpha' method achieves the lowest MSE.](352c5fab6f936356e9570761a02ab71e_img.jpg)

Figure 6: The MSE on different graphs, varying alpha. The figure contains four bar charts (a) Ca-HepPh, (b) Cit-HepPh, (c) Twitter, and (d) Com-DBLP. Each chart plots MSE (y-axis) against epsilon (x-axis) for four alpha values: 0.3, 0.6, 0.9, and best alpha. In all cases, the 'best alpha' method achieves the lowest MSE.

Fig. 6. The MSE on different graphs, varying  $\alpha$

![Figure 7: The runtime on different graphs. The figure contains five bar charts (a) Ca-HepPh, (b) Cit-HepPh, (c) Twitter, (d) Twitter, and (e) Com-DBLP. Each chart plots Time (s) on a logarithmic y-axis against epsilon (x-axis) for four methods: PureLDP + NodeProj, CryptoAssisted + NodeProj, PureLDP + EdgeProj, and CryptoAssisted + EdgeProj. The 'PureLDP + NodeProj' method is consistently the fastest across all datasets.](91be14371a97fb5ce9eeb29ae18d07c3_img.jpg)

Figure 7: The runtime on different graphs. The figure contains five bar charts (a) Ca-HepPh, (b) Cit-HepPh, (c) Twitter, (d) Twitter, and (e) Com-DBLP. Each chart plots Time (s) on a logarithmic y-axis against epsilon (x-axis) for four methods: PureLDP + NodeProj, CryptoAssisted + NodeProj, PureLDP + EdgeProj, and CryptoAssisted + EdgeProj. The 'PureLDP + NodeProj' method is consistently the fastest across all datasets.

Fig. 7. The runtime on different graphs

### C. Impact of privacy budget allocation

To further estimate the influence of the privacy allocation scheme on the overall utility, we compare the best  $\alpha$  with other three constant  $\alpha$ , including 0.3, 0.6, and 0.9. We present the MSE results of different  $\alpha$  on different graph datasets in Fig. 6. We can observe that the best  $\alpha$  owns the lowest MSE against the other allocation schemes in most cases. On the other hand, with the increase of the overall privacy budget  $\epsilon$ , the MSE value is decreasing. Thus most of privacy budget can be allocated to the final publishing the degree distribution, which is roughly consistent with our best  $\alpha$  in Table II, namely,  $\epsilon_3$  for publishing degree distribution is approximately equal to the overall privacy budget  $\epsilon$ .

### D. Analysis on running time

Finally, we compare the running time overhead of our proposed methods, as shown in Fig. 7. We can see that the running time of ‘CryptoAssisted+EdgeProj’ method is much larger than that of ‘PureLDP+NodeProj’ method. This is mainly because edge-level projection method needs to traverse each edge of every node and crypto-assisted parameter selection method has  $n$  users to communicate in pairs, which is in line with our theoretical analysis in Section VI. The difference between ‘CryptoAssisted+EdgeProj’ method and ‘PureLDP+NodeProj’ method is larger on Twitter. This is because Twitter has more edges than other graphs, as described in Table V, which results in higher computation and communication overhead.

## VIII. RELATED WORK

There are many existing works related to Node-CDP and Edge-LDP.

**Node-CDP.** There have been many prior research works related to Node differential privacy (Node-DP). For example, a handful of graph algorithms [10]–[13] have been designed for publishing the degree distribution by proposing different graph

projection methods. For instance, the truncation method [11] removes all nodes with the degree over  $\theta$ . Edge-removal approach [13] traverses all edges in an arbitrary order and removes each edge connected to a node with a degree more than  $\theta$ . Edge-addition method [10] traverses the edges in a stable order and inserts each edge correlated to node with degree over  $\theta$ . However, the existing projection methods are only designed for Node-CDP and are not viable in Node-LDP.

**Edge-LDP.** Since there is no need for a trusted server and a large amount of valuable information resides in a decentralized social network, LDP is becoming increasingly popular in privacy protection of graph analysis. Existing works focus on various graph statistics, such as degree distribution (or histogram) [23], subgraph counting (e.g.,  $k$ -clique,  $k$ -star,  $k$ -triangle) [19], [35], synthetic graph generation [18], [36], publishing attributed graph [17], [37], etc. For instance, Ye *et al.* [23] propose a LDP-enabled graph metric estimation framework for general graph analysis. In [19], subgraph counting is protected locally by a more sophisticated algorithm that uses an additional round of interaction between individuals and server. To strike a balance between noise added to satisfy LDP and information loss from a coarser granularity, Qin *et al.* [18] design a novel multi-phase approach to synthetic decentralized social graph generation. However, these existing works are all based on Edge-LDP which provides a weaker privacy guarantee than our work under Node-LDP.

## IX. CONCLUSION

To conclude, we first discuss the motivation for publishing the graph statistics under Node-LDP, and present the challenges of finishing the projection locally. We propose two methods for the projection parameter selection: pureLDP parameter selection and crypto-assisted parameter selection. Also, we design two methods for executing local graph projection: node-level local projection and edge-level local

projection. Theoretical and experimental analysis verify the utility and privacy achieved by our proposed work.

## ACKNOWLEDGMENT

This work was partially supported by JST SPRING JP-MJSP2110, JST CREST JPMJCR21M2, JST SICORP JP-MJSC2107, JSPS KAKENHI Grant Numbers 21K19767, 22H03595, 22H00521.

## REFERENCES

- [1] V. Karwa, S. Raskhodnikova, A. Smith, and G. Yaroslavtsev, "Private analysis of graph structure," *Proc. VLDB Endow.*, vol. 4, no. 11, p. 1146–1157, aug 2011. [Online]. Available: <https://doi.org/10.14778/3402707.3402749>
- [2] C. Dwork, A. Roth *et al.*, "The algorithmic foundations of differential privacy," *Found. Trends Theor. Comput. Sci.*, vol. 9, no. 3-4, pp. 211–407, 2014.
- [3] N. Li, M. Lyu, D. Su, and W. Yang, "Differential privacy: From theory to practice," *Synthesis Lectures on Information Security, Privacy, & Trust*, vol. 8, no. 4, pp. 1–138, 2016.
- [4] C. Task and C. Clifton, "A guide to differential privacy theory in social network analysis," in *2012 IEEE/ACM International Conference on Advances in Social Networks Analysis and Mining*. IEEE, 2012, pp. 411–417.
- [5] Y. Li, M. Purcell, T. Rakotoarivelo, D. Smith, T. Ranbaduge, and K. S. Ng, "Private graph data release: A survey," *arXiv preprint arXiv:2107.04245*, 2021.
- [6] Q. Qian, Z. Li, P. Zhao, W. Chen, H. Yin, and L. Zhao, "Publishing graph node strength histogram with edge differential privacy," in *International Conference on Database Systems for Advanced Applications*. Springer, 2018, pp. 75–91.
- [7] M. Hay, C. Li, G. Miklau, and D. Jensen, "Accurate estimation of the degree distribution of private networks," in *2009 Ninth IEEE International Conference on Data Mining*. IEEE, 2009, pp. 169–178.
- [8] V. Karwa and A. B. Slavković, "Differentially private graphical degree sequences and synthetic graphs," in *International Conference on Privacy in Statistical Databases*. Springer, 2012, pp. 273–285.
- [9] D. Proserpio, S. Goldberg, and F. McSherry, "A workflow for differentially-private graph synthesis," in *Proceedings of the 2012 ACM workshop on Workshop on online social networks*, 2012, pp. 13–18.
- [10] W.-Y. Day, N. Li, and M. Lyu, "Publishing graph degree distribution with node differential privacy," in *Proceedings of the 2016 International Conference on Management of Data*, 2016, pp. 123–138.
- [11] S. P. Kasiviswanathan, K. Nissim, S. Raskhodnikova, and A. Smith, "Analyzing graphs with node differential privacy," in *Theory of Cryptography Conference*. Springer, 2013, pp. 457–476.
- [12] S. Raskhodnikova and A. Smith, "Lipschitz extensions for node-private graph statistics and the generalized exponential mechanism," in *2016 IEEE 57th Annual Symposium on Foundations of Computer Science (FOCS)*. IEEE, 2016, pp. 495–504.
- [13] J. Blocki, A. Blum, A. Datta, and O. Sheffet, "Differentially private data analysis of social networks via restricted sensitivity," in *Proceedings of the 4th conference on Innovations in Theoretical Computer Science*, 2013, pp. 87–96.
- [14] M. Yang, L. Lyu, J. Zhao, T. Zhu, and K.-Y. Lam, "Local differential privacy and its applications: A comprehensive survey," *arXiv preprint arXiv:2008.03686*, 2020.
- [15] J. C. Duchi, M. I. Jordan, and M. J. Wainwright, "Local privacy and statistical minimax rates," in *2013 IEEE 54th Annual Symposium on Foundations of Computer Science*. IEEE, 2013, pp. 429–438.
- [16] S. P. Kasiviswanathan, H. K. Lee, K. Nissim, S. Raskhodnikova, and A. Smith, "What can we learn privately?" *SIAM Journal on Computing*, vol. 40, no. 3, pp. 793–826, 2011.
- [17] C. Wei, S. Ji, C. Liu, W. Chen, and T. Wang, "Asgldp: Collecting and generating decentralized attributed graphs with local differential privacy," *IEEE Transactions on Information Forensics and Security*, vol. 15, pp. 3239–3254, 2020.
- [18] Z. Qin, T. Yu, Y. Yang, I. Khalil, X. Xiao, and K. Ren, "Generating synthetic decentralized social graphs with local differential privacy," in *Proceedings of the 2017 ACM SIGSAC Conference on Computer and Communications Security*, 2017, pp. 425–438.
- [19] J. Imola, T. Murakami, and K. Chaudhuri, "Locally differentially private analysis of graph statistics," in *30th USENIX Security Symposium (USENIX Security 21)*, 2021, pp. 983–1000.
- [20] M. A. Kamara and X. Li, "A review of order preserving encryption schemes," in *The International Conference on Natural Computation, Fuzzy Systems and Knowledge Discovery*. Springer, 2020, pp. 707–715.
- [21] A. Tueno and F. Kerschbaum, "Efficient secure computation of order-preserving encryption," in *Proceedings of the 15th ACM Asia Conference on Computer and Communications Security*, 2020, pp. 193–207.
- [22] K. Bonawitz, V. Ivanov, B. Kreuter, A. Marcedone, H. B. McMahan, S. Patel, D. Ramage, A. Segal, and K. Seth, "Practical secure aggregation for privacy-preserving machine learning," in *proceedings of the 2017 ACM SIGSAC Conference on Computer and Communications Security*, 2017, pp. 1175–1191.
- [23] Q. Ye, H. Hu, M. H. Au, X. Meng, and X. Xiao, "Lf-gdpr: A framework for estimating graph metrics with local differential privacy," *IEEE Transactions on Knowledge and Data Engineering*, 2020.
- [24] C. J. Willmott and K. Matsuura, "Advantages of the mean absolute error (mae) over the root mean square error (rmse) in assessing average model performance," *Climate research*, vol. 30, no. 1, pp. 79–82, 2005.
- [25] H. Zhang, S. Latif, R. Bassily, and A. Rountev, "Differentially-private control-flow node coverage for software usage analysis," in *USENIX Security Symposium (USENIX Security)*, 2020.
- [26] D. Kifer and A. Machanavajjhala, "No free lunch in data privacy," in *Proceedings of the 2011 ACM SIGMOD International Conference on Management of Data*, ser. SIGMOD '11. New York, NY, USA: Association for Computing Machinery, 2011, p. 193–204. [Online]. Available: <https://doi.org/10.1145/1989323.1989345>
- [27] R. A. Popa, F. H. Li, and N. Zeldovich, "An ideal-security protocol for order-preserving encoding," in *2013 IEEE Symposium on Security and Privacy*. IEEE, 2013, pp. 463–477.
- [28] J. Leskovec and A. Krevl, "SNAP Datasets: Stanford large network dataset collection," <http://snap.stanford.edu/data>, Jun. 2014.
- [29] R. Agrawal, J. Kiernan, R. Srikanth, and Y. Xu, "Order preserving encryption for numeric data," in *Proceedings of the 2004 ACM SIGMOD international conference on Management of data*, 2004, pp. 563–574.
- [30] A. Boldyreva, N. Chenette, and A. O'Neill, "Order-preserving encryption revisited: Improved security analysis and alternative solutions," in *Annual Cryptology Conference*. Springer, 2011, pp. 578–595.
- [31] F. Kerschbaum and A. Schröpfer, "Optimal average-complexity ideal-security order-preserving encryption," in *Proceedings of the 2014 ACM SIGSAC Conference on Computer and Communications Security*, 2014, pp. 275–286.
- [32] D. S. Roche, D. Apon, S. G. Choi, and A. Yerukhimovich, "Pope: Partial order preserving encoding," in *Proceedings of the 2016 ACM SIGSAC Conference on Computer and Communications Security*, 2016, pp. 1131–1142.
- [33] D. Liu and S. Wang, "Programmable order-preserving secure index for encrypted database query," in *2012 IEEE Fifth International Conference on Cloud Computing*, 2012, pp. 502–509.
- [34] M. Blum and S. Micali, "How to generate cryptographically strong sequences of pseudorandom bits," *SIAM journal on Computing*, vol. 13, no. 4, pp. 850–864, 1984.
- [35] H. Sun, X. Xiao, I. Khalil, Y. Yang, Z. Qin, H. Wang, and T. Yu, "Analyzing subgraph statistics from extended local views with decentralized differential privacy," in *Proceedings of the 2019 ACM SIGSAC Conference on Computer and Communications Security*, 2019, pp. 703–717.
- [36] Y. Zhang, J. Wei, X. Zhang, X. Hu, and W. Liu, "A two-phase algorithm for generating synthetic graph under local differential privacy," in *Proceedings of the 8th International Conference on Communication and Network Security*, 2018, pp. 84–89.
- [37] Z. Jorgensen, T. Yu, and G. Cormode, "Publishing attributed social graphs with formal privacy guarantees," in *Proceedings of the 2016 international conference on management of data*, 2016, pp. 107–122.